use crate::instructions::extensions::ExtensionDiscriminator;
use crate::instructions::MAX_MULTISIG_SIGNERS;
use core::mem::MaybeUninit;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use solana_account_view::AccountView;
use solana_address::Address;
use solana_instruction_view::cpi::Signer;
use solana_instruction_view::{
    cpi::invoke_signed_with_bounds, InstructionAccount, InstructionView,
};
use solana_program_error::{ProgramError, ProgramResult};

/// Pause a mint, preventing all token operations.
///
/// ### Accounts:
///
/// **Single authority**
///   0. `[WRITE]` The mint to pause.
///   1. `[SIGNER]` The mint's pause authority.
///
/// **Multisignature authority**
///   0. `[WRITE]` The mint to pause.
///   1. `[]` The multisig account that is the pause authority.
///   2. `[SIGNER]` M signer accounts (as required by the multisig).
pub struct Pause<'a, 'b, 'c> {
    /// The mint to pause
    pub mint: &'a AccountView,
    /// The mint's pause authority (single or multisig)
    pub pause_authority: &'a AccountView,
    /// Signer accounts if the authority is a multisig
    pub signers: &'c [&'a AccountView],
    /// Token Program
    pub token_program: &'b Address,
}

impl Pause<'_, '_, '_> {
    pub const DISCRIMINATOR: u8 = 1;

    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    #[inline(always)]
    pub fn invoke_signed(&self, signers: &[Signer]) -> ProgramResult {
        let &Self {
            mint,
            pause_authority,
            signers: multisig_accounts,
            token_program,
            ..
        } = self;

        if multisig_accounts.len() > MAX_MULTISIG_SIGNERS {
            return Err(ProgramError::InvalidArgument);
        }

        const UNINIT_INSTRUCTION_ACCOUNTS: MaybeUninit<InstructionAccount> =
            MaybeUninit::<InstructionAccount>::uninit();
        let mut instruction_accounts = [UNINIT_INSTRUCTION_ACCOUNTS; 2 + MAX_MULTISIG_SIGNERS];

        unsafe {
            // Index 0: The mint (writable)
            instruction_accounts
                .get_unchecked_mut(0)
                .write(InstructionAccount::writable(mint.address()));

            // Index 1: The pause authority (signer if single, readonly if multisig)
            instruction_accounts
                .get_unchecked_mut(1)
                .write(InstructionAccount::new(
                    pause_authority.address(),
                    false,
                    multisig_accounts.is_empty(),
                ));
        }

        // Multisig signers
        for (instruction_account, signer) in instruction_accounts[2..]
            .iter_mut()
            .zip(multisig_accounts.iter())
        {
            instruction_account.write(InstructionAccount::readonly_signer(signer.address()));
        }

        // Instruction data layout:
        // -  [0]: token instruction discriminator (PausableExtension)
        // -  [1]: extension sub-instruction (Pause = 1)
        let instruction_data = [ExtensionDiscriminator::Pausable as u8, Self::DISCRIMINATOR];

        let num_accounts = 2 + multisig_accounts.len();

        let instruction = InstructionView {
            program_id: token_program,
            accounts: unsafe { from_raw_parts(instruction_accounts.as_ptr() as _, num_accounts) },
            data: &instruction_data,
        };

        // Gather all accounts for invoke_signed_with_bounds
        let mut all_accounts = [None; 2 + MAX_MULTISIG_SIGNERS];
        all_accounts[0] = Some(mint);
        all_accounts[1] = Some(pause_authority);

        for (i, signer) in multisig_accounts.iter().enumerate() {
            all_accounts[2 + i] = Some(*signer);
        }

        invoke_signed_with_bounds::<{ 2 + MAX_MULTISIG_SIGNERS }>(
            &instruction,
            unsafe { from_raw_parts_mut(all_accounts.as_mut_ptr() as _, num_accounts) },
            signers,
        )?;

        Ok(())
    }
}
