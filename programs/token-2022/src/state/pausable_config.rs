use solana_account_view::{AccountView, Ref};
use solana_address::Address;
use solana_program_error::ProgramError;

use crate::ID;

/// Pausable configuration for a mint.
///
/// Indicates that the tokens from this mint can be paused. When paused,
/// minting, transferring, and burning tokens is disallowed.
#[repr(C)]
pub struct PausableConfig {
    /// Authority that can pause or resume activity on the mint.
    authority: Address,

    /// Whether minting / transferring / burning tokens is paused.
    /// Value is 1 when paused, 0 when active.
    paused: u8,
}

impl PausableConfig {
    /// The length of the `PausableConfig` data.
    pub const LEN: usize = core::mem::size_of::<PausableConfig>();

    /// Return a `PausableConfig` from the given account view.
    ///
    /// This method performs owner and length validation on `AccountView`, safe borrowing
    /// the account data.
    #[inline]
    pub fn from_account_view(
        account_view: &AccountView,
    ) -> Result<Ref<PausableConfig>, ProgramError> {
        if account_view.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if !account_view.owned_by(&ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(Ref::map(account_view.try_borrow()?, |data| unsafe {
            Self::from_bytes_unchecked(data)
        }))
    }

    /// Return a `PausableConfig` from the given account view.
    ///
    /// This method performs owner and length validation on `AccountView`, but does not
    /// perform the borrow check.
    ///
    /// # Safety
    ///
    /// The caller must ensure that it is safe to borrow the account data (e.g., there are
    /// no mutable borrows of the account data).
    #[inline]
    pub unsafe fn from_account_view_unchecked(
        account_view: &AccountView,
    ) -> Result<&PausableConfig, ProgramError> {
        if account_view.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if account_view.owner() != &ID {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(Self::from_bytes_unchecked(account_view.borrow_unchecked()))
    }

    /// Return a `PausableConfig` from the given bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `bytes` contains a valid representation of `PausableConfig`,
    /// and it is properly aligned to be interpreted as an instance of `PausableConfig`.
    /// At the moment `PausableConfig` has an alignment of 1 byte.
    /// This method does not perform a length validation.
    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        &*(bytes[..Self::LEN].as_ptr() as *const PausableConfig)
    }

    /// Return the authority that can pause or resume the mint.
    #[inline(always)]
    pub fn authority(&self) -> &Address {
        &self.authority
    }

    /// Return whether the mint is currently paused.
    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.paused != 0
    }
}
