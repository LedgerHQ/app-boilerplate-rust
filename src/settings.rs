use ledger_device_sdk::nvm::*;
use ledger_device_sdk::NVMData;

// This is necessary to store the object in NVM and not in RAM
const SETTINGS_SIZE: usize = 10;
#[link_section = ".nvm_data"]
static mut DATA: NVMData<AtomicStorage<[u8; SETTINGS_SIZE]>> =
    NVMData::new(AtomicStorage::new(&[0u8; 10]));

#[derive(Clone, Copy)]
pub struct Settings;

impl Default for Settings {
    fn default() -> Self {
        Settings
    }
}

impl Settings {
    #[inline(never)]
    #[allow(unused)]
    pub fn get_mut_ref(&mut self) -> &mut AtomicStorage<[u8; SETTINGS_SIZE]> {
        return unsafe { DATA.get_mut() };
    }

    #[allow(unused)]
    pub fn get_element(&self, index: usize) -> u8 {
        let settings = unsafe { DATA.get_mut() };
        settings.get_ref()[index]
    }

    #[allow(unused)]
    // Not used in this boilerplate, but can be used to set a value in the settings
    pub fn set_element(&self, index: usize, value: u8) {
        let mut updated_data: [u8; SETTINGS_SIZE] = unsafe { *DATA.get_mut().get_ref() };
        updated_data[index] = value;
        unsafe {
            DATA.get_mut().update(&updated_data);
        }
    }
}
