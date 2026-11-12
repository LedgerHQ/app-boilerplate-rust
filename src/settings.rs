use ledger_device_sdk::nvm::*;

const SETTINGS_SIZE: usize = 10;
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
        unsafe { (*data).get_mut() }
    }

    #[inline(never)]
    #[allow(unused)]
    pub fn get_ref(&mut self) -> &AtomicStorage<[u8; SETTINGS_SIZE]> {
        let data = &raw const DATA;
        unsafe { (*data).get_ref() }
    }

    #[allow(unused)]
    pub fn get_element(&self, index: usize) -> u8 {
        let data = &raw const DATA;
        let storage = unsafe { (*data).get_ref() };
        let settings = storage.get_ref();
        settings[index]
    }

    #[allow(unused)]
    // Not used in this boilerplate, but can be used to set a value in the settings
    pub fn set_element(&self, index: usize, value: u8) {
        let data = &raw mut DATA;
        let storage = unsafe { (*data).get_mut() };
        let mut updated_data = *storage.get_ref();
        updated_data[index] = value;
        unsafe {
            storage.update(&updated_data);
        }
    }
}
