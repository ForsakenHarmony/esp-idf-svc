use core::cmp::min;
use core::ptr;

extern crate alloc;
use alloc::sync::Arc;

use embedded_svc::storage::{RawStorage, StorageBase};

use esp_idf_sys::*;

use crate::{nvs::*, private::cstr::*};

enum EspNvsRef {
    Default(Arc<EspDefaultNvs>),
    Nvs(Arc<EspNvs>),
}

pub struct EspNvsStorage(EspNvsRef, NvsHandle);

impl EspNvsStorage {
    pub fn new_default(
        default_nvs: Arc<EspDefaultNvs>,
        namespace: impl AsRef<str>,
        read_write: bool,
    ) -> Result<Self, EspError> {
        let handle = default_nvs.open(namespace, read_write)?;

        Ok(Self(EspNvsRef::Default(default_nvs), handle))
    }

    pub fn new(
        nvs: Arc<EspNvs>,
        namespace: impl AsRef<str>,
        read_write: bool,
    ) -> Result<Self, EspError> {
        let handle = nvs.open(namespace, read_write)?;

        Ok(Self(EspNvsRef::Nvs(nvs), handle))
    }
}

impl StorageBase for EspNvsStorage {
    type Error = EspError;

    fn contains(&self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        match self.1.get_u64(key) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) if e.code() == ESP_ERR_NVS_INVALID_LENGTH as i32 => Ok(true),
            Err(e) => Err(e),
        }
    }

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        let res = self.1.erase_key(key)?;
        self.1.commit()?;

        Ok(res)
    }
}

impl RawStorage for EspNvsStorage {
    fn len(&self, name: &str) -> Result<Option<usize>, Self::Error> {
        let c_key = CString::new(name).unwrap();

        let mut value: u_int64_t = 0;

        // check for u64 value
        match unsafe { nvs_get_u64(self.1, c_key.as_ptr(), &mut value as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => {
                // check for blob value, by getting blob length
                let mut len: size_t = 0;
                match unsafe {
                    nvs_get_blob(self.1, c_key.as_ptr(), ptr::null_mut(), &mut len as *mut _)
                } {
                    ESP_ERR_NVS_NOT_FOUND => Ok(None),
                    err => {
                        // bail on error
                        esp!(err)?;

                        Ok(Some(len as _))
                    }
                }
            }
            err => {
                // bail on error
                esp!(err)?;

                // u64 value was found, decode it
                let len: u8 = (value & 0xff) as u8;

                Ok(Some(len as _))
            }
        }
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<vec::Vec<u8>>, Self::Error> {
        let key = key.as_ref();

        match self.1.get_u64(key) {
            Ok(None) => Ok(None),
            Err(e) if e.code() == ESP_ERR_NVS_INVALID_LENGTH as i32 => self.1.get_blob(key),
            Ok(Some(mut value)) => {
                let len: u8 = (value & 0xff) as u8;
                value >>= 8;

                let array: [u8; 7] = [
                    (value & 0xff) as u8,
                    ((value >> 8) & 0xff) as u8,
                    ((value >> 16) & 0xff) as u8,
                    ((value >> 24) & 0xff) as u8,
                    ((value >> 32) & 0xff) as u8,
                    ((value >> 48) & 0xff) as u8,
                    ((value >> 56) & 0xff) as u8,
                ];

                Ok(Some(array[..len as usize].to_vec()))
            }
            Err(e) => Err(e),
        }
    }

    fn put_raw(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<vec::Vec<u8>>,
    ) -> Result<bool, Self::Error> {
        let key = key.as_ref();
        let value = value.into();

        // start by just clearing this key
        self.1.erase_key(key)?;

        if value.len() < 8 {
            let uvalue = 0;

            for v in value.iter().rev() {
                uvalue <<= 8;
                uvalue |= *v as u_int64_t;
            }

            uvalue <<= 8;
            uvalue |= value.len() as u_int64_t;

            self.1.set_u64(key, uvalue)?;
        } else {
            self.1.set_blob(key, value)?;
        }

        self.1.commit()?;

        Ok(found)
    }
}

// TODO
// impl Storage for EspNvsStorage {
//     fn get<'a, T>(&'a self, name: &str) -> Result<Option<T>, Self::Error>
//     where
//         T: serde::Deserialize<'a> {
//         todo!()
//     }

//     fn set<T>(&mut self, name: &str, value: &T) -> Result<bool, Self::Error>
//     where
//         T: serde::Serialize {
//         todo!()
//     }
// }
