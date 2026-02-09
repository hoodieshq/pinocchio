use crate::instructions::extensions::ExtensionDiscriminator;
use crate::instructions::MAX_MULTISIG_SIGNERS;
use core::mem::MaybeUninit;
use core::slice;
use solana_account_view::AccountView;
use solana_address::Address;
use solana_instruction_view::cpi::Signer;
use solana_instruction_view::{
    cpi::invoke_signed_with_bounds, InstructionAccount, InstructionView,
};
use solana_program_error::{ProgramError, ProgramResult};

/// Resume a paused mint, allowing token operations again.
///
/// ### Accounts:
///
/// **Single authority**
///   0. `[WRITE]` The mint to resume.
///   1. `[SIGNER]` The mint's pause authority.
///
/// **Multisignature authority**
///   0. `[WRITE]` The mint to resume.
///   1. `[]` The multisig account that is the pause authority.
///   2. `[SIGNER]` M signer accounts (as required by the multisig).
pub struct Resume<'a, 'b, 'c> {
    /// The mint to resume
    pub mint: &'a AccountView,
    /// The mint's pause authority (single or multisig)
    pub pause_authority: &'a AccountView,
    /// Signer accounts if the authority is a multisig
    pub signers: &'c [&'a AccountView],
    /// Token Program
    pub token_program: &'b Address,
}

impl Resume<'_, '_, '_> {
    pub const DISCRIMINATOR: u8 = 2;

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
        // -  [1]: extension sub-instruction (Resume = 2)
        let instruction_data = [ExtensionDiscriminator::Pausable as u8, Self::DISCRIMINATOR];

        let num_accounts = 2 + multisig_accounts.len();

        let instruction = InstructionView {
            program_id: token_program,
            accounts: unsafe {
                slice::from_raw_parts(instruction_accounts.as_ptr() as _, num_accounts)
            },
            data: &instruction_data,
        };

        // Account view array
        const UNINIT_ACCOUNT_VIEWS: MaybeUninit<&AccountView> = MaybeUninit::uninit();
        let mut account_views = [UNINIT_ACCOUNT_VIEWS; 2 + MAX_MULTISIG_SIGNERS];

        unsafe {
            // SAFETY:
            // - `account_views` is sized to 2 + MAX_MULTISIG_SIGNERS
            // - Index 0 is always present
            account_views.get_unchecked_mut(0).write(mint);
            // - Index 1 is always present
            account_views.get_unchecked_mut(1).write(pause_authority);
        }

        // Fill signer accounts
        for (account_view, signer) in account_views[2..].iter_mut().zip(multisig_accounts.iter()) {
            account_view.write(signer);
        }

        invoke_signed_with_bounds::<{ 2 + MAX_MULTISIG_SIGNERS }>(
            &instruction,
            unsafe {
                slice::from_raw_parts(account_views.as_ptr() as *const &AccountView, num_accounts)
            },
            signers,
        )
    }
}
