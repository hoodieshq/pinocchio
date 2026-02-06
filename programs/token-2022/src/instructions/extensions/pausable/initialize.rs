use crate::instructions::extensions::ExtensionDiscriminator;
use crate::{write_bytes, UNINIT_BYTE};
use core::slice::from_raw_parts;
use solana_account_view::AccountView;
use solana_address::Address;
use solana_instruction_view::{cpi::invoke, InstructionAccount, InstructionView};
use solana_program_error::ProgramResult;

/// Initialize the pausable extension for a mint.
///
/// ### Accounts:
///   0. `[WRITE]` The mint to initialize.
pub struct InitializePausable<'a, 'b> {
    /// The mint to initialize the pausable config
    pub mint: &'a AccountView,
    /// The public key for the account that can pause or resume activity on the mint
    pub authority: &'a Address,
    /// Token Program
    pub token_program: &'b Address,
}

impl InitializePausable<'_, '_> {
    pub const DISCRIMINATOR: u8 = 0;

    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        let instruction_accounts = [InstructionAccount::writable(self.mint.address())];

        // Instruction data layout:
        // -  [0]: token instruction discriminator (PausableExtension)
        // -  [1]: extension sub-instruction (Initialize = 0)
        // -  [2..34]: authority pubkey (32 bytes)

        let mut instruction_data = [UNINIT_BYTE; 34];

        write_bytes(
            &mut instruction_data[0..1],
            &[ExtensionDiscriminator::Pausable as u8],
        );
        write_bytes(&mut instruction_data[1..2], &[Self::DISCRIMINATOR]);
        write_bytes(&mut instruction_data[2..34], &self.authority.to_bytes());

        let instruction = InstructionView {
            program_id: self.token_program,
            accounts: &instruction_accounts,
            data: unsafe { from_raw_parts(instruction_data.as_ptr() as _, 34) },
        };

        invoke(&instruction, &[self.mint])
    }
}
