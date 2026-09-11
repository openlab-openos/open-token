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

pub(crate) async fn eco_process_command(
    matches: &ArgMatches<'_>,
    config: &Config<'_>,
    mut signers: Vec<Arc<dyn Signer>>,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
) -> CommandResult {
    assert!(!config.sign_only);

    match matches.subcommand() {
        ("create", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let meta = value_t_or_exit!(arg_matches, "meta", String);
            let body = value_t_or_exit!(arg_matches, "body", String);
            let years = value_t_or_exit!(arg_matches, "years", u8);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_create(config, signers, name, meta, body, years, &owner, &program_id).await?;
        }
        ("update-meta", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let meta = value_t_or_exit!(arg_matches, "meta", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_update_meta(config, signers, name, meta, &owner, &program_id).await?;
        }
        ("update-body", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let body = value_t_or_exit!(arg_matches, "body", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_update_body(config, signers, name, body, &owner, &program_id).await?;
        }
        ("renew-ownership", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let years = value_t_or_exit!(arg_matches, "years", u8);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_renew_ownership(config, signers, name, years, &owner, &program_id).await?;
        }
        ("transfer-ownership", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let to_pubkey = pubkey_of_signer(arg_matches, "to", wallet_manager)
                .expect("Required argument 'to' missing or invalid");
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_transfer_ownership(config, signers, name, to_pubkey, &owner, &program_id).await?;
        }
        ("sale-ask", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let enabled = value_t_or_exit!(arg_matches, "enabled", u8);
            let sell_price = value_t_or_exit!(arg_matches, "sell_price", u64);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_sale_ask(config, signers, name, enabled, sell_price, &owner, &program_id).await?;
        }
        ("sale-bid", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_sale_bid(config, signers, name, &owner, &program_id).await?;
        }
        ("rent-ask", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let enabled = value_t_or_exit!(arg_matches, "enabled", u8);
            let rent_per_day = value_t_or_exit!(arg_matches, "rent_per_day", u64);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_rent_ask(config, signers, name, enabled, rent_per_day, &owner, &program_id).await?;
        }
        ("rent-bid", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let days = value_t_or_exit!(arg_matches, "days", u32);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_rent_bid(config, signers, name, days, &owner, &program_id).await?;
        }
        ("update-rent", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let rent_info = value_t_or_exit!(arg_matches, "rent-info", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_update_rent(config, signers, name, rent_info, &owner, &program_id).await?;
        }
        ("renew-usership", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let days = value_t_or_exit!(arg_matches, "days", u32);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_renew_usership(config, signers, name, days, &owner, &program_id).await?;
        }
        ("transfer-usership", Some(arg_matches)) => {
            let name = value_t_or_exit!(arg_matches, "name", String);
            let to_pubkey = pubkey_of_signer(arg_matches, "to", wallet_manager)
                .expect("Required argument 'to' missing or invalid");
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(ONS_PROGRAM_ID).unwrap();
            command_transfer_usership(config, signers, name, to_pubkey, &owner, &program_id).await?;
        }
        _ => unreachable!(),
    }

    Ok("".to_string())
}

async fn command_create(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    meta: String,
    body: String,
    years: u8,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    if years < 1 {
        return Err(format!("Years must be >= 1").into());
    }
    println!("ecosystem create...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);
    let (treasury_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);
    let (tpl_pubkey, _tpl_bump) = find_pda(&[b"eco_tpl"], program_id);

    let treasury_account = rpc_client.get_account(&treasury_pubkey).await
        .map_err(|_| format!("Treasury config account does not exist: {}", treasury_pubkey))?;
    let treasury_data = &treasury_account.data;
    if treasury_data.len() < 40 {
        return Err(format!("Treasury account data too short").into());
    }    
    let json_data = &treasury_data[40..];    
    if json_data.len() < 4 {
        return Err(format!("Invalid treasury JSON data: too short").into());
    }    
    let json_len = u32::from_le_bytes([json_data[0], json_data[1], json_data[2], json_data[3]]) as usize;    
    if json_data.len() < 4 + json_len {
        return Err(format!("Invalid treasury JSON length").into());
    }    
    let json_str = std::str::from_utf8(&json_data[4..4 + json_len])
        .map_err(|_| format!("Invalid UTF-8 in treasury config_json"))?;
    let treasury_config: TreasuryConfig = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse treasury config: {}", e))?;
    let fee_receiver_pubkey = Pubkey::from_str(&treasury_config.fee_receiver)
        .map_err(|_| format!("Invalid fee_receiver in treasury config"))?;

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(body_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(treasury_pubkey, false),
        AccountMeta::new_readonly(tpl_pubkey, false),
        AccountMeta::new(fee_receiver_pubkey, false), //fee_receiver
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_create"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    data.extend_from_slice(meta.as_bytes());

    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(body.as_bytes());

    data.push(years);

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem created. hash: {}", signature);
    Ok(())
}

async fn command_update_meta(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    meta: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    println!("ecosystem update meta...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (tpl_pubkey, _tpl_bump) = find_pda(&[b"eco_tpl"], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new_readonly(tpl_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_meta_update"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    data.extend_from_slice(meta.as_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem update meta finished. hash: {}", signature);
    Ok(())
}

async fn command_update_body(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    body: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    println!("ecosystem update body...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(body_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_body_update"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(body.as_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem update body finished. hash: {}", signature);
    Ok(())
}

async fn command_renew_ownership(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    years: u8,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    if years < 1 {
        return Err(format!("Years must be >= 1").into());
    }
    println!("ecosystem ownership renew...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (treasury_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);

    let treasury_account = rpc_client.get_account(&treasury_pubkey).await
        .map_err(|_| format!("Treasury config account does not exist: {}", treasury_pubkey))?;
    let treasury_data = &treasury_account.data;
    if treasury_data.len() < 40 {
        return Err(format!("Treasury account data too short").into());
    }    
    let json_data = &treasury_data[40..];    
    if json_data.len() < 4 {
        return Err(format!("Invalid treasury JSON data: too short").into());
    }    
    let json_len = u32::from_le_bytes([json_data[0], json_data[1], json_data[2], json_data[3]]) as usize;    
    if json_data.len() < 4 + json_len {
        return Err(format!("Invalid treasury JSON length").into());
    }    
    let json_str = std::str::from_utf8(&json_data[4..4 + json_len])
        .map_err(|_| format!("Invalid UTF-8 in treasury config_json"))?;
    let treasury_config: TreasuryConfig = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse treasury config: {}", e))?;
    let fee_receiver_pubkey = Pubkey::from_str(&treasury_config.fee_receiver)
        .map_err(|_| format!("Invalid fee_receiver in treasury config"))?;

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(treasury_pubkey, false),
        AccountMeta::new(fee_receiver_pubkey, false), //fee_receiver
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_owner_renew"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.push(years);

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem ownership renew finished. hash: {}", signature);
    Ok(())
}

async fn command_transfer_ownership(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    to: Option<Pubkey>,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    println!("ecosystem ownership transfer...");
    let to_pbk = to.ok_or_else(|| "Error: '--to' is required but was not provided")?;
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(body_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new(to_pbk, false), // to
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false)
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_owner_transfer"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&to_pbk.to_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem ownership transfer finished. hash: {}", signature);
    Ok(())
}

async fn command_sale_ask(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    enabled: u8,
    sell_price: u64,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    println!("ecosystem sale ask...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_sale_ask"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.push(enabled);
    data.extend_from_slice(&sell_price.to_le_bytes()); //u64

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem sale ask finished. hash: {}", signature);
    Ok(())
}

async fn command_sale_bid(
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
    println!("ecosystem sale bid...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);

    let meta_account = rpc_client.get_account(&meta_pubkey).await
        .map_err(|_| format!("Meta config account does not exist: {}", meta_pubkey))?;
    let meta_data = &meta_account.data;
    if meta_data.len() < 40 {
        return Err(format!("Meta account data too short").into());
    }    
    let json_data = &meta_data[40..];    
    if json_data.len() < 4 {
        return Err(format!("Invalid meta JSON data: too short").into());
    }    
    let json_len = u32::from_le_bytes([json_data[0], json_data[1], json_data[2], json_data[3]]) as usize;    
    if json_data.len() < 4 + json_len {
        return Err(format!("Invalid JSON length").into());
    }    
    let json_str = std::str::from_utf8(&json_data[4..4 + json_len])
        .map_err(|_| format!("Invalid UTF-8 in meta config_json"))?;
    let meta_config: MetaConfig = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse meta config: {}", e))?;
    let fee_receiver_pubkey = Pubkey::from_str(&meta_config.fee_receiver)
        .map_err(|_| format!("Invalid fee_receiver in meta config"))?;
    let pay_token_str = &meta_config.pay_token.unwrap();    
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(body_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new(fee_receiver_pubkey, false), //fee_receiver
        AccountMeta::new_readonly(token2022_program_pbk, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
    ];
    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(pay_token_str)?;
        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            payer_pbk, 
            &mint_pubkey,
            &token2022_program_pbk
        );
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fee_receiver_pubkey,
            &mint_pubkey, 
            &token2022_program_pbk   
        );
        accounts.push(AccountMeta::new(from_ata, false));
        accounts.push(AccountMeta::new(to_ata, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
    }else{
        let mint_pubkey = Pubkey::from_str("Cmdnkd1MJBfKuBjp3j33BqeesZCYrnJ4mXnk19Uhs3z2")?;
        let ata = Pubkey::from_str("6gymrEA7R98Y1dffrTng5BCstmxjgdN692DxRsFtoMvM")?;
        accounts.push(AccountMeta::new(ata, false));
        accounts.push(AccountMeta::new(ata, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
    } 

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_sale_bid"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem sale bid finished. hash: {}", signature);
    Ok(())
}

async fn command_rent_ask(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    enabled: u8,
    rent_per_day: u64,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    println!("ecosystem rent ask...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_rent_ask"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.push(enabled);
    data.extend_from_slice(&rent_per_day.to_le_bytes()); //u64

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem rent ask finished. hash: {}", signature);
    Ok(())
}

async fn command_rent_bid(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    days: u32,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    println!("ecosystem rent bid...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (rent_pubkey, _rent_bump) = find_pda(&[b"rent", &name_hash], program_id);

    let meta_account = rpc_client.get_account(&meta_pubkey).await
        .map_err(|_| format!("Meta config account does not exist: {}", meta_pubkey))?;
    let meta_data = &meta_account.data;
    if meta_data.len() < 40 {
        return Err(format!("Meta account data too short").into());
    }    
    let json_data = &meta_data[40..];    
    if json_data.len() < 4 {
        return Err(format!("Invalid meta JSON data: too short").into());
    }    
    let json_len = u32::from_le_bytes([json_data[0], json_data[1], json_data[2], json_data[3]]) as usize;    
    if json_data.len() < 4 + json_len {
        return Err(format!("Invalid JSON length").into());
    }    
    let json_str = std::str::from_utf8(&json_data[4..4 + json_len])
        .map_err(|_| format!("Invalid UTF-8 in meta config_json"))?;
    let meta_config: MetaConfig = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse meta config: {}", e))?;
    let fee_receiver_pubkey = Pubkey::from_str(&meta_config.fee_receiver)
        .map_err(|_| format!("Invalid fee_receiver in meta config"))?;
    let pay_token_str = &meta_config.pay_token.unwrap();    
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();
    
    let mut accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(rent_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new(fee_receiver_pubkey, false), //fee_receiver
    ];

    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(pay_token_str)?;
        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            payer_pbk, 
            &mint_pubkey,
            &token2022_program_pbk
        );
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fee_receiver_pubkey,
            &mint_pubkey, 
            &token2022_program_pbk   
        );
        accounts.push(AccountMeta::new(from_ata, false));
        accounts.push(AccountMeta::new(to_ata, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
    }else{
        let mint_pubkey = Pubkey::from_str("Cmdnkd1MJBfKuBjp3j33BqeesZCYrnJ4mXnk19Uhs3z2")?;
        let ata = Pubkey::from_str("6gymrEA7R98Y1dffrTng5BCstmxjgdN692DxRsFtoMvM")?;
        accounts.push(AccountMeta::new(ata, false));
        accounts.push(AccountMeta::new(ata, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
    } 
    accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false));
    accounts.push(AccountMeta::new_readonly(spl_associated_token_account::ID, false));

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_rent_bid"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&days.to_le_bytes()); //u32

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem rent bid finished. hash: {}", signature);
    Ok(())
}

async fn command_update_rent(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    rent_info: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    println!("ecosystem update rent...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (rent_pubkey, _rent_bump) = find_pda(&[b"rent", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(rent_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_rent_info_update"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&(rent_info.len() as u32).to_le_bytes());
    data.extend_from_slice(rent_info.as_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem update rent finished. hash: {}", signature);
    Ok(())
}

async fn command_renew_usership(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    days: u32,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    if days < 1 {
        return Err(format!("Days must be >= 1").into());
    }
    println!("ecosystem usership renew...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);

    let meta_account = rpc_client.get_account(&meta_pubkey).await
        .map_err(|_| format!("Meta config account does not exist: {}", meta_pubkey))?;
    let meta_data = &meta_account.data;
    if meta_data.len() < 40 {
        return Err(format!("Meta account data too short").into());
    }    
    let json_data = &meta_data[40..];    
    if json_data.len() < 4 {
        return Err(format!("Invalid meta JSON data: too short").into());
    }    
    let json_len = u32::from_le_bytes([json_data[0], json_data[1], json_data[2], json_data[3]]) as usize;    
    if json_data.len() < 4 + json_len {
        return Err(format!("Invalid JSON length").into());
    }    
    let json_str = std::str::from_utf8(&json_data[4..4 + json_len])
        .map_err(|_| format!("Invalid UTF-8 in meta config_json"))?;
    let meta_config: MetaConfig = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse meta config: {}", e))?;
    let fee_receiver_pubkey = Pubkey::from_str(&meta_config.fee_receiver)
        .map_err(|_| format!("Invalid fee_receiver in meta config"))?;
    let pay_token_str = &meta_config.pay_token.unwrap();    
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(fee_receiver_pubkey, false), //fee_receiver
    ];
    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(pay_token_str)?;
        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            payer_pbk, 
            &mint_pubkey,
            &token2022_program_pbk
        );
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fee_receiver_pubkey,
            &mint_pubkey, 
            &token2022_program_pbk   
        );
        accounts.push(AccountMeta::new(from_ata, false));
        accounts.push(AccountMeta::new(to_ata, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
    }else{
        let mint_pubkey = Pubkey::from_str("Cmdnkd1MJBfKuBjp3j33BqeesZCYrnJ4mXnk19Uhs3z2")?;
        let ata = Pubkey::from_str("6gymrEA7R98Y1dffrTng5BCstmxjgdN692DxRsFtoMvM")?;
        accounts.push(AccountMeta::new(ata, false));
        accounts.push(AccountMeta::new(ata, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
    } 
    accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false));
    accounts.push(AccountMeta::new_readonly(spl_associated_token_account::ID, false));

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_rent_renew"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&days.to_le_bytes()); //u32

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem usership renew finished. hash: {}", signature);
    Ok(())
}

async fn command_transfer_usership(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    name: String,
    to: Option<Pubkey>,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;    
    let name_len = name.len();
    if name_len == 0 || name_len > 100 {
        return Err(format!("Name length must be 1-100").into());
    }
    println!("ecosystem usership transfer...");
    let to_pbk = to.ok_or_else(|| "Error: '--to' is required but was not provided")?;
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (rent_pubkey, _rent_bump) = find_pda(&[b"rent", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(rent_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new(to_pbk, false), // to
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false)
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("eco_rent_transfer"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&to_pbk.to_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = simulation.value.err {
        for log in simulation.value.logs.unwrap_or_default() {
            println!("{}", log);
        }
        return Err(format!("Simulation failed: {:?}", err).into());
    }
         
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;
         
    println!("ecosystem usership transfer finished. hash: {}", signature);
    Ok(())
}
