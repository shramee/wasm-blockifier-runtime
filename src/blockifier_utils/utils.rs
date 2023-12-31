pub use blockifier::abi::abi_utils::{get_storage_var_address, selector_from_name};
pub use blockifier::block_context::BlockContext;
pub use blockifier::execution::contract_class::{ContractClass, ContractClassV1};
pub use blockifier::execution::entry_point::{
    CallEntryPoint, CallInfo, EntryPointExecutionContext, ExecutionResources,
};
pub use blockifier::execution::errors::EntryPointExecutionError;
pub use blockifier::stdlib::collections::HashMap;
pub use blockifier::transaction::account_transaction::AccountTransaction;
pub use blockifier::transaction::errors::TransactionExecutionError;
pub use blockifier::transaction::objects::{AccountTransactionContext, TransactionExecutionInfo};
use cairo_vm_const::*;
use serde_json::Value;
pub use starknet_api::api_core::{ChainId, ClassHash, ContractAddress, Nonce, PatriciaKey};
pub use starknet_api::block::{BlockNumber, BlockTimestamp};
pub use starknet_api::hash::{StarkFelt, StarkHash};
pub use starknet_api::state::StorageKey;
use starknet_api::transaction::TransactionHash;
pub use starknet_api::transaction::{Calldata, Fee, InvokeTransactionV1, TransactionSignature};
pub mod cairo_vm_const {
    pub const OUTPUT_BUILTIN_NAME: &str = "output_builtin";
    pub const HASH_BUILTIN_NAME: &str = "pedersen_builtin";
    pub const RANGE_CHECK_BUILTIN_NAME: &str = "range_check_builtin";
    pub const SIGNATURE_BUILTIN_NAME: &str = "ecdsa_builtin";
    pub const BITWISE_BUILTIN_NAME: &str = "bitwise_builtin";
    pub const EC_OP_BUILTIN_NAME: &str = "ec_op_builtin";
    pub const KECCAK_BUILTIN_NAME: &str = "keccak_builtin";
    pub const POSEIDON_BUILTIN_NAME: &str = "poseidon_builtin";
    pub const SEGMENT_ARENA_BUILTIN_NAME: &str = "segment_arena_builtin";
}
use core::convert::{TryFrom, TryInto};

/// Fee token contract - Contract Address and Class Hash (identical)
pub const FEE_TKN_ADDR: &str = "0x1";

/// Account contract class hash, can also be deployed to same address
pub const ACCOUNT_ADDR: &str = "0x100";

/// Universal Deployer class hash, can also be deployed to same address
pub const DEPLOYER_ADDR: &str = "0x2";

pub const DEFAULT_GAS_PRICE: u128 = 100 * u128::pow(10, 9); // Given in units of wei.
pub const CAIRO_STEPS: u32 = 1_000_000;

// The max_fee used for txs in this test.
pub const MAX_FEE: u128 = 1000000 * 100000000000; // 1000000 * min_gas_price.

pub mod addr {
    use super::*;
    pub fn patricia(key: &str) -> PatriciaKey {
        PatriciaKey(felt(key))
    }
    pub fn felt(hex: &str) -> StarkFelt {
        hex.try_into().unwrap()
    }
    pub fn class(class_hash_str: &str) -> ClassHash {
        ClassHash(felt(class_hash_str))
    }

    pub fn contract(contract_addr_str: &str) -> ContractAddress {
        ContractAddress(patricia(contract_addr_str))
    }

    pub fn storage(storage_var_name: &str, args: &[&str]) -> StorageKey {
        let args: Vec<StarkFelt> = args.iter().map(|a| addr::felt(a)).collect();
        get_storage_var_address(storage_var_name, &args).unwrap()
    }

    pub fn key(storage_key: &str) -> StorageKey {
        StorageKey(patricia(storage_key))
    }
}

pub fn invoke_calldata(contract: &str, entry_point: &str, calldata: Vec<&str>) -> Calldata {
    let entry_point_selector = selector_from_name(entry_point);
    let mut calldata_with_callee = vec![
        addr::felt(contract),   // Contract address.
        entry_point_selector.0, // EP selector.
    ];
    calldata_with_callee.push(addr::felt(&calldata.len().to_string()));
    for param in calldata.into_iter() {
        calldata_with_callee.push(param.try_into().unwrap());
    }

    Calldata(calldata_with_callee.into())
}

pub fn compile_sierra_class(json: &str) -> Result<ContractClass, String> {
    let value: Value = serde_json::from_str(json).unwrap();

    let contract_class = cairo_lang_starknet::contract_class::ContractClass {
        abi: serde_json::from_value(value["abi"].clone()).ok(),
        sierra_program: serde_json::from_value(value["sierra_program"].clone()).unwrap(),
        entry_points_by_type: serde_json::from_value(value["entry_points_by_type"].clone())
            .unwrap(),
        contract_class_version: serde_json::from_value(value["contract_class_version"].clone())
            .unwrap(),
        sierra_program_debug_info: serde_json::from_value(
            value["sierra_program_debug_info"].clone(),
        )
        .ok(),
    };

    match contract_class.into_casm_contract_class(false) {
        Ok(casm_class) => Ok(ContractClass::V1(ContractClassV1::try_from(casm_class).unwrap())),
        Err(e) => Err(format!("{:#?}", e)),
    }
}

pub fn invoke_tx(
    sender_address: &str,
    calldata: Calldata,
    signature: Option<TransactionSignature>,
    nonce: &str,
) -> AccountTransaction {
    AccountTransaction::Invoke(blockifier::transaction::transactions::InvokeTransaction {
        tx: starknet_api::transaction::InvokeTransaction::V1(InvokeTransactionV1 {
            max_fee: Fee(MAX_FEE),
            sender_address: addr::contract(sender_address),
            calldata,
            signature: signature.unwrap_or_default(),
            nonce: Nonce(addr::felt(nonce)),
            ..Default::default()
        }),
        tx_hash: TransactionHash::default(),
    })
}

pub fn block_context() -> BlockContext {
    let vm_resource_fee_cost = HashMap::from([
        ("n_steps".to_string(), 1_f64),
        (HASH_BUILTIN_NAME.to_string(), 1_f64),
        (RANGE_CHECK_BUILTIN_NAME.to_string(), 1_f64),
        (SIGNATURE_BUILTIN_NAME.to_string(), 1_f64),
        (BITWISE_BUILTIN_NAME.to_string(), 1_f64),
        (POSEIDON_BUILTIN_NAME.to_string(), 1_f64),
        (OUTPUT_BUILTIN_NAME.to_string(), 1_f64),
        (EC_OP_BUILTIN_NAME.to_string(), 1_f64),
    ])
    .into();

    BlockContext {
        chain_id: ChainId("DOJO_CLIENT".to_string()),
        block_number: BlockNumber::default(),
        block_timestamp: BlockTimestamp::default(),
        sequencer_address: addr::contract("0x01"),
        fee_token_address: addr::contract(FEE_TKN_ADDR),
        vm_resource_fee_cost,
        gas_price: DEFAULT_GAS_PRICE,
        invoke_tx_max_n_steps: CAIRO_STEPS,
        validate_max_n_steps: CAIRO_STEPS,
        max_recursion_depth: 100,
    }
}
