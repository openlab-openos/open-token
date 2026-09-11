/// The `entity` subcommand
use {
    crate::{clap_app::Error, command::CommandResult, config::Config}, clap::{value_t_or_exit, ArgMatches}, futures::future::ok, solana_clap_utils::input_parsers::pubkey_of_signer, solana_client::{
        nonblocking::rpc_client::RpcClient, rpc_client::RpcClient as BlockingRpcClient,
        tpu_client::{TpuClient, TpuClientConfig},
    }, solana_remote_wallet::remote_wallet::RemoteWalletManager, solana_sdk::{
        message::Message, native_token::{lamports_to_sol, Sol}, program_pack::Pack, pubkey::Pubkey, 
        signature::Signer, signer, system_instruction, instruction::{AccountMeta, Instruction}, transaction::Transaction,
    }, spl_associated_token_account::*, spl_token_2022::{
        extension::StateWithExtensions,
        instruction,
        state::{Account, Mint},
    }, std::{rc::Rc, sync::Arc, time::Instant
    }, crate::utils::{hash_name, find_pda, ONS_PROGRAM_ID, TreasuryConfig, MetaConfig, instruction_discriminator, 
        HeaderInfoJson, AuthJsonConfigJson, ONS_API_URL,
    }, solana_program::{pubkey::Pubkey as SolPubkey
    }, std::str::FromStr,
    serde_json::Value,
};

pub(crate) async fn ons_process_command(
    matches: &ArgMatches<'_>,
    config: &Config<'_>,
    mut signers: Vec<Arc<dyn Signer>>,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
) -> CommandResult {
    assert!(!config.sign_only);

    match matches.subcommand() {        
        ("get-info", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_get_info(config, signers, name, &owner, &program_id).await?;
        }
        ("get-list-by-owner", Some(arg_matches)) => {
            let owner_address = value_t_or_exit!(arg_matches, "owner_address", String);
            let page = value_t_or_exit!(arg_matches, "page", u32);
            let page_size = value_t_or_exit!(arg_matches, "page_size", u32);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_get_list_by_owner(config, signers, owner_address, page, page_size, &owner, &program_id).await?;
        }
        ("get-list-by-parent", Some(arg_matches)) => {
            let parent_name = value_t_or_exit!(arg_matches, "parent_name", String);
            let page = value_t_or_exit!(arg_matches, "page", u32);
            let page_size = value_t_or_exit!(arg_matches, "page_size", u32);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_get_list_by_parent(config, signers, parent_name, page, page_size, &owner, &program_id).await?;
        }
        ("get-all-wildcard-names", Some(arg_matches)) => {
            let wildcard_name = value_t_or_exit!(arg_matches, "wildcard_name", String);
            let parent_name = value_t_or_exit!(arg_matches, "parent_name", String);
            let page = value_t_or_exit!(arg_matches, "page", u32);
            let page_size = value_t_or_exit!(arg_matches, "page_size", u32);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_get_all_wildcard_name(config, signers, wildcard_name, parent_name, page, page_size, &owner, &program_id).await?;
        }
        _ => unreachable!(),
    }

    Ok("".to_string())
}

async fn command_get_info(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    println!("get information...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);
    #[derive(serde::Serialize, Debug)]
    struct Info {
        exist: u8,
        header: Option<HeaderInfoJson>,
        meta: Option<Value>,
        body: Option<Value>,
    }

    let mut info = Info {
        exist: 0,
        header: None,
        meta: None,
        body: None,
    };
    match rpc_client.get_account(&header_pubkey).await {
        Ok(account) => {            
            if let Some(header_info) = HeaderInfoJson::deserialize(&account.data) {
                info.header = Some(header_info);
                info.exist = 1;
            } else {
                eprintln!("Failed to deserialize header");
                info.exist = 0;
            }
        }
        Err(_) => {
            info.exist = 0;
        }
    }
    if info.exist != 0 {
        match rpc_client.get_account(&meta_pubkey).await {
            Ok(account) => {
                if let Some(meta_config) = AuthJsonConfigJson::deserializee(&account.data) {
                    info.meta = Some(meta_config);
                } else {
                    eprintln!("Failed to deserialize meta");
                }
            }
            Err(_) => {
            }
        }
    }

    if info.exist != 0 {
        match rpc_client.get_account(&body_pubkey).await {
            Ok(account) => {
                if let Some(body_config) = AuthJsonConfigJson::deserializee(&account.data) {
                    info.body = Some(body_config);
                } else {
                    println!("Failed to deserialize body");
                }
            }
            Err(_) => {
                println!("Failed to deserialize body");
            }
        }
    }
    println!("info: {}", serde_json::to_string_pretty(&info).unwrap());
    Ok(())
}


async fn command_get_list_by_owner(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    owner_address: String,
    page: u32,
    page_size: u32,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    println!("get list by owner...");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(3000))
        .build()?;

    let response = client
        .get(format!("{}/names/getListByOwner", ONS_API_URL))
        .query(&[
            ("owner", owner_address),
            ("page", page.to_string()),
            ("pageSize", page_size.to_string()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    let json_data: Value = response.json().await?;
    println!("{}", serde_json::to_string_pretty(&json_data)?);
    Ok(())
}

async fn command_get_list_by_parent(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    parent_name: String,
    page: u32,
    page_size: u32,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    println!("get list by parent...");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(3000))
        .build()?;

    let response = client
        .get(format!("{}/names/getListByParent", ONS_API_URL))
        .query(&[
            ("parent", parent_name),
            ("page", page.to_string()),
            ("pageSize", page_size.to_string()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    let json_data: Value = response.json().await?;
    println!("{}", serde_json::to_string_pretty(&json_data)?);
    Ok(())
}

async fn command_get_all_wildcard_name(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    wildcard_name: String,
    parent_name: String,
    page: u32,
    page_size: u32,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    println!("get all wildcard names...");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(3000))
        .build()?;

    let response = client
        .get(format!("{}/names/getWildcardNames", ONS_API_URL))
        .query(&[
            ("name", wildcard_name),
            ("parent", parent_name),
            ("page", page.to_string()),
            ("pageSize", page_size.to_string()),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    let json_data: Value = response.json().await?;
    println!("{}", serde_json::to_string_pretty(&json_data)?);
    Ok(())
}
