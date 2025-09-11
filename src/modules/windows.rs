// extern crate winapi;

// use winapi::um::securitybaseapi::CheckTokenMembership;
// use winapi::um::winnt::{
//     DOMAIN_ALIAS_RID_ADMINS, SECURITY_BUILTIN_DOMAIN_RID, SECURITY_NT_AUTHORITY, SID_IDENTIFIER_AUTHORITY,
// };
// use winapi::shared::minwindef::FALSE;
// use winapi::shared::ntdef::NULL;
// use std::ptr::null_mut;

// use anyhow::{Result, anyhow};

// /// Returns if the current process is elevated or not.
// /// 
// /// If any error occures, it will fail with Err(e).
// /// Not memory safe, as doing literally anything with the windows API will be unsafe by nature.
// pub fn is_process_elevated() -> Result<bool> {
//     let mut administrators_group: winapi::um::winnt::PSID = null_mut();
//     let nt_authority = SID_IDENTIFIER_AUTHORITY {
//         Value: SECURITY_NT_AUTHORITY,
//     };

//     let nt_authority_ptr = &nt_authority as *const SID_IDENTIFIER_AUTHORITY as *mut SID_IDENTIFIER_AUTHORITY;

//     let success = unsafe {
//         winapi::um::securitybaseapi::AllocateAndInitializeSid(
//             nt_authority_ptr,
//             2,
//             SECURITY_BUILTIN_DOMAIN_RID,
//             DOMAIN_ALIAS_RID_ADMINS,
//             0, 0, 0, 0, 0, 0,
//             &mut administrators_group as *mut _ as *mut _
//         )
//     };

//     if success != FALSE {
//         let mut is_member = FALSE;
//         let token_membership_result = unsafe {
//             CheckTokenMembership(NULL, administrators_group, &mut is_member)
//         };
        
//         if token_membership_result == FALSE {
//             // Failed to check membership, defaulting to non-administrator.
//             return Err(anyhow!("Failed to check current user membership privileges."));
//         }

//         unsafe { winapi::um::securitybaseapi::FreeSid(administrators_group); }
//         Ok(is_member != FALSE)
//     } else {
//         Ok(false)
//     }
// }