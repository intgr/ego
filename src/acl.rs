use core::mem;
use std::ffi::CString;
use std::io::Error;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::slice::from_raw_parts;
use std::str::from_utf8;

use acl_sys::*;
use libc::ssize_t;
use libc::types::common::c95::c_void;
use simple_error::{SimpleError, SimpleResult};

use crate::acl::Qualifier::*;

pub struct PosixACL {
    acl: acl_t,
}

/** NB! Unix-only */
fn path_to_cstring(path: &Path) -> CString {
    CString::new(path.as_os_str().as_bytes()).unwrap()
}

#[allow(dead_code)]
pub enum Qualifier {
    Undefined,
    UserObj,
    GroupObj,
    Other,
    User(u32),
    Group(u32),
    Mask,
}

impl Qualifier {
    fn tag_type(&self) -> i32 {
        match self {
            Undefined => ACL_UNDEFINED_TAG,
            UserObj => ACL_USER_OBJ,
            GroupObj => ACL_GROUP_OBJ,
            User(_) => ACL_USER,
            Group(_) => ACL_GROUP,
            Mask => ACL_MASK,
            Other => ACL_OTHER,
        }
    }
    fn uid(&self) -> Option<u32> {
        match self {
            User(uid) | Group(uid) => Some(*uid),
            _ => None,
        }
    }
}

fn check_return(ret: i32, func: &str) {
    if ret != 0 {
        panic!("Error in {}: {}", func, Error::last_os_error());
    }
}

impl PosixACL {
    pub fn read_acl(path: &Path) -> Result<PosixACL, SimpleError> {
        let c_path = path_to_cstring(path);
        let acl: acl_t = unsafe { acl_get_file(c_path.as_ptr(), ACL_TYPE_ACCESS) };
        if acl.is_null() {
            bail!(
                "Error reading {} ACL: {}",
                path.display(),
                Error::last_os_error()
            );
        }
        Ok(PosixACL { acl })
    }

    pub fn write_acl(&mut self, path: &Path) -> SimpleResult<()> {
        let c_path = path_to_cstring(path);
        self.fix_mask();
        self.validate()?;
        let ret = unsafe { acl_set_file(c_path.as_ptr(), ACL_TYPE_ACCESS, self.acl) };
        if ret != 0 {
            bail!(
                "Error writing {} ACL: {}",
                path.display(),
                Error::last_os_error()
            );
        }
        Ok(())
    }

    pub fn set(&mut self, qual: Qualifier, perm: u32) {
        unsafe {
            let mut entry: acl_entry_t = mem::zeroed();
            check_return(
                acl_create_entry(&mut self.acl, &mut entry),
                "acl_create_entry",
            );
            let mut permset: acl_permset_t = mem::zeroed();
            check_return(acl_get_permset(entry, &mut permset), "acl_get_permset");
            check_return(acl_add_perm(permset, perm), "acl_add_perm");
            check_return(acl_set_permset(entry, permset), "acl_set_permset");
            check_return(acl_set_tag_type(entry, qual.tag_type()), "acl_set_tag_type");
            if let Some(uid) = qual.uid() {
                check_return(
                    acl_set_qualifier(entry, &uid as *const u32 as *const c_void),
                    "acl_set_qualifier",
                );
            }
        }
    }

    pub fn fix_mask(&mut self) {
        unsafe {
            check_return(acl_calc_mask(&mut self.acl), "acl_calc_mask");
        }
    }

    pub fn as_text(&self) -> String {
        let chars = unsafe {
            let mut len: ssize_t = 0;
            let txt = acl_to_text(self.acl, &mut len);
            if txt.is_null() {
                panic!("Error in acl_to_text: {}", Error::last_os_error());
            }
            from_raw_parts(txt as *const u8, len as usize)
        };
        from_utf8(chars).unwrap().to_string()
    }

    pub fn compact_text(&self) -> String {
        self.as_text().replace('\n', ",")
    }

    pub fn validate(&self) -> SimpleResult<()> {
        let ret = unsafe { acl_valid(self.acl) };
        if ret != 0 {
            bail!("Invalid ACL: {}", self.compact_text());
        }
        Ok(())
    }
}
