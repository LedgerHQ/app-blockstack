#![allow(non_camel_case_types, non_snake_case)]
#![allow(clippy::cast_ptr_alignment)]

use crate::parser::{
    parser_common::ParserError, post_condition::TransactionPostCondition, transaction::Transaction,
};

// extern c function for formatting to fixed point number
extern "C" {
    pub fn fp_uint64_to_str(out: *mut i8, outLen: u16, value: u64, decimals: u8) -> u16;
}

#[repr(C)]
#[no_mangle]
pub struct parser_context_t {
    pub buffer: *const u8,
    pub bufferLen: u16,
    pub offset: u16,
}

#[repr(C)]
#[no_mangle]
pub struct parse_tx_t {
    state: *mut u8,
    len: u16,
}

fn transaction_from<'a>(tx: *mut parse_tx_t) -> Option<&'a mut Transaction<'a>> {
    unsafe { ((*tx).state as *const u8 as *mut Transaction).as_mut() }
}

#[no_mangle]
pub extern "C" fn _parser_init(
    ctx: *mut parser_context_t,
    buffer: *const u8,
    bufferSize: u16,
    alloc_size: *mut u16,
) -> u32 {
    // Lets the caller know how much memory we need for allocating
    // our global state
    if alloc_size.is_null() {
        return ParserError::parser_no_memory_for_state as u32;
    }
    unsafe {
        *alloc_size = core::mem::size_of::<Transaction>() as u16;
    }
    parser_init_context(ctx, buffer, bufferSize) as u32
}

fn parser_init_context(
    ctx: *mut parser_context_t,
    buffer: *const u8,
    bufferSize: u16,
) -> ParserError {
    unsafe {
        (*ctx).offset = 0;

        if bufferSize == 0 || buffer.is_null() {
            (*ctx).buffer = core::ptr::null_mut();
            (*ctx).bufferLen = 0;
            return ParserError::parser_init_context_empty;
        }

        (*ctx).buffer = buffer;
        (*ctx).bufferLen = bufferSize;
        ParserError::parser_ok
    }
}

#[no_mangle]
pub extern "C" fn _read(context: *const parser_context_t, parser_state: *mut parse_tx_t) -> u32 {
    let data = unsafe { core::slice::from_raw_parts((*context).buffer, (*context).bufferLen as _) };

    if let Some(tx) = transaction_from(parser_state) {
        match tx.read(data) {
            Ok(_) => ParserError::parser_ok as u32,
            Err(e) => e as u32,
        }
    } else {
        ParserError::parser_no_memory_for_state as u32
    }
}

#[no_mangle]
pub extern "C" fn _getNumItems(_ctx: *const parser_context_t, tx_t: *const parse_tx_t) -> u8 {
    unsafe {
        if tx_t.is_null() || (*tx_t).state.is_null() {
            return 0;
        }
    }
    if let Some(tx) = transaction_from(tx_t as _) {
        return tx.num_items();
    }
    0
}

#[no_mangle]
pub extern "C" fn _getItem(
    _ctx: *const parser_context_t,
    displayIdx: u8,
    outKey: *mut i8,
    outKeyLen: u16,
    outValue: *mut i8,
    outValueLen: u16,
    pageIdx: u8,
    pageCount: *mut u8,
    tx_t: *const parse_tx_t,
) -> u32 {
    let (page_count, key, value) = unsafe {
        *pageCount = 0u8;
        let page_count = &mut *pageCount;
        let key = core::slice::from_raw_parts_mut(outKey as *mut u8, outKeyLen as usize);
        let value = core::slice::from_raw_parts_mut(outValue as *mut u8, outValueLen as usize);
        if tx_t.is_null() || (*tx_t).state.is_null() {
            return ParserError::parser_context_mismatch as _;
        }
        (page_count, key, value)
    };
    if let Some(tx) = transaction_from(tx_t as _) {
        match tx.get_item(displayIdx, key, value, pageIdx) {
            Ok(page) => {
                *page_count = page;
                ParserError::parser_ok as _
            }
            Err(e) => e as _,
        }
    } else {
        ParserError::parser_context_mismatch as _
    }
}

#[no_mangle]
pub extern "C" fn _auth_flag(tx_t: *const parse_tx_t, auth_flag: *mut u8) -> u32 {
    if let Some(tx) = transaction_from(tx_t as _) {
        unsafe {
            *auth_flag = tx.auth_flag() as u8;
            ParserError::parser_ok as _
        }
    } else {
        ParserError::parser_context_mismatch as _
    }
}

#[no_mangle]
pub extern "C" fn _fee_bytes(tx_t: *const parse_tx_t, fee: *mut u8, fee_len: u16) -> u8 {
    if let Some(tx) = transaction_from(tx_t as _) {
        unsafe {
            let fee_bytes = if let Some(fee) = tx.fee() {
                fee.to_be_bytes()
            } else {
                return 0;
            };

            if fee_bytes.len() <= fee_len as usize {
                fee.copy_from(fee_bytes.as_ptr(), fee_bytes.len());
                return fee_bytes.len() as u8;
            }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn _nonce_bytes(tx_t: *const parse_tx_t, nonce: *mut u8, nonce_len: u16) -> u8 {
    if let Some(tx) = transaction_from(tx_t as _) {
        unsafe {
            let nonce_bytes = if let Some(nonce) = tx.nonce() {
                nonce.to_be_bytes()
            } else {
                return 0;
            };

            if nonce_bytes.len() <= nonce_len as usize {
                nonce.copy_from(nonce_bytes.as_ptr(), nonce_bytes.len());
                return nonce_bytes.len() as u8;
            }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn _check_pubkey_hash(
    tx_t: *const parse_tx_t,
    pubKey: *const u8,
    pubKeyLen: u16,
) -> u8 {
    if let Some(tx) = transaction_from(tx_t as _) {
        unsafe {
            if pubKey.is_null() {
                return ParserError::parser_no_data as _;
            }
            let pk = core::slice::from_raw_parts(pubKey, pubKeyLen as _);
            tx.check_signer_pk_hash(pk) as _
        }
    } else {
        ParserError::parser_context_mismatch as _
    }
}

#[no_mangle]
pub extern "C" fn _presig_hash_data(tx_t: *const parse_tx_t, buf: *mut u8, bufLen: u16) -> u16 {
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf, bufLen as usize) };

    if let Some(tx) = transaction_from(tx_t as _) {
        if let Ok(len) = tx.transaction_auth.initial_sighash_auth(buffer) {
            return len as _;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn _last_block_ptr(tx_t: *const parse_tx_t) -> *const u8 {
    if let Some(tx) = transaction_from(tx_t as _) {
        return tx.last_transaction_block();
    }
    core::ptr::null()
}
