use ledger_device_sdk::nvm::*;

const SETTINGS_SIZE: usize = 10;

/// Index of the switch adding the derivation path to the transaction review.
///
/// A setting may only add information to a review: every signed field is always displayed.
pub const SETTING_SHOW_PATH: usize = 0;
ledger_device_sdk::nvm_data! {
    static mut DATA: AtomicStorage<[u8; SETTINGS_SIZE]> = AtomicStorage::new(&[0u8; SETTINGS_SIZE]);
}

#[derive(Clone, Copy)]
pub struct Settings;

impl Default for Settings {
    fn default() -> Self {
        Settings
    }
}

impl Settings {
    /// The settings storage, for `nvm_settings`.
    ///
    /// # Safety
    /// No other reference to the storage may be in use while the returned one is: call it
    /// once per settings screen, and do not read the storage through another reference
    /// while that screen is shown.
    #[inline(never)]
    #[allow(unused)]
    pub unsafe fn get_mut(&mut self) -> &'static mut AtomicStorage<[u8; SETTINGS_SIZE]> {
        let data = &raw mut DATA;
        // SAFETY: the pointer to the static is valid and aligned, and the caller guarantees that
        // no other reference to the storage is in use while the returned one is.
        unsafe { (*data).get_mut() }
    }

    #[inline(never)]
    #[allow(unused)]
    pub fn get_ref(&mut self) -> &AtomicStorage<[u8; SETTINGS_SIZE]> {
        let data = &raw const DATA;
        // SAFETY: the pointer to the static is valid and aligned, and the app is single-threaded:
        // mutable references only exist under the contracts of `get_mut` and `set_element`.
        unsafe { (*data).get_ref() }
    }

    #[allow(unused)]
    pub fn get_element(&self, index: usize) -> u8 {
        let data = &raw const DATA;
        // SAFETY: as in `get_ref`.
        let storage = unsafe { (*data).get_ref() };
        let settings = storage.get_ref();
        settings[index]
    }

    /// Stores `value` at `index`. Not used in this boilerplate, where settings are only changed
    /// from the settings screen.
    ///
    /// # Safety
    /// Same contract as [`Self::get_mut`]: must not be called while the settings screen, or any
    /// other reference to the storage, is in use.
    #[allow(unused)]
    pub unsafe fn set_element(&self, index: usize, value: u8) {
        let data = &raw mut DATA;
        // SAFETY: the pointer to the static is valid and aligned, and the caller guarantees that
        // no other reference to the storage is in use while this one is.
        let storage = unsafe { (*data).get_mut() };
        let mut updated_data = *storage.get_ref();
        updated_data[index] = value;
        storage.update(&updated_data);
    }
}
