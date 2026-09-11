/// The `entity` subcommand
use {
    crate::{clap_app::Error, command::CommandResult, config::Config, 
        utils::{find_pda, hash_name, instruction_discriminator, MetaConfig, TreasuryConfig, ONS_PROGRAM_ID, HeaderInfoJson, AuthJsonConfigJson,
    }}, clap::{value_t_or_exit, ArgMatches, Error as OtherError}, solana_clap_utils::input_parsers::pubkey_of_signer, solana_client::{
        nonblocking::rpc_client::RpcClient, rpc_client::RpcClient as BlockingRpcClient,
        tpu_client::{TpuClient, TpuClientConfig},
    }, solana_program::pubkey::Pubkey as SolPubkey, solana_remote_wallet::remote_wallet::RemoteWalletManager, 
    solana_sdk::{
        fee, instruction::{AccountMeta, Instruction}, message::Message, native_token::{lamports_to_sol, Sol}, program_pack::Pack, 
        pubkey::Pubkey, signature::Signer, system_instruction, transaction::Transaction, compute_budget::ComputeBudgetInstruction,
    }, spl_associated_token_account::*, spl_token_2022::{
        extension::StateWithExtensions,
        instruction,
        state::{Account, Mint},
    }, std::{rc::Rc, str::FromStr, sync::Arc, time::Instant
    }, serde_json::Value,
};

pub(crate) async fn entity_process_command(
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

pub fn require_valid_entity_name(label: &str) -> Result<(), Error> {
    if label.is_empty() {
        return Err(format!("empty name").into());
    }
    if label.starts_with('-') {
        return Err(format!("Invalid character").into());
    }
    if label.ends_with('-') {
        return Err(format!("Invalid character").into());
    }
    for ch in label.chars() {
        if !ch.is_ascii() || !(ch.is_ascii_alphanumeric() || ch == '-') {
            return Err(format!("invalid character").into());
        }
    }
    Ok(())
}

pub fn parse_enity_full_name(uri: &str) -> Result<Vec<String>, Error> {
    const PROTOCOL_SEPARATOR: &str = "://";
    
    let separator_index = uri.find(PROTOCOL_SEPARATOR)
        .ok_or(format!("missing ecosystem separator"))?;
    
    let protocol = &uri[..separator_index];
    if protocol.is_empty() {
        return Err(format!("invalid entity name").into()); 
    }

    let host_part_start = separator_index + PROTOCOL_SEPARATOR.len();
    let host_part = &uri[host_part_start..];
    if host_part.is_empty() {
        return Err(format!("invalid entity name").into());
    }
    for label in host_part.split('.') {
        require_valid_entity_name(label)?;
    }
    if host_part.is_empty() || host_part.starts_with('.') || host_part.ends_with('.') {
        return Err(format!("invalid entity name").into());
    }
    let parts: Vec<&str> = host_part.split('.').collect();
    if parts.is_empty() || parts.iter().any(|p| p.is_empty()) {
        return Err(format!("invalid entity name").into());
    }
    let mut parent_names = Vec::new();
    let total_parts = parts.len();
    if total_parts > 5 {
        return Err(format!("Max entity level is 4").into());
    }    
    parent_names.push(protocol.to_string());
    for i in (0..total_parts).rev() {
        let parent_name = parts[i..].join(".");
        let full_parent_name = format!("{}://{}", protocol, parent_name);
        parent_names.push(full_parent_name);
    }
    //parent_names.push(parts[0].to_string());
    Ok(parent_names)
}

pub fn extract_json_str(data: &[u8]) -> Result<&str, Error> {
    const HEADER_LEN: usize = 44;

    if data.len() < HEADER_LEN {
        return Err(format!("Data length ({}) < 44", data.len()).into());
    }

    let json_len = u32::from_le_bytes([data[40], data[41], data[42], data[43]]) as usize;
    let json_end = HEADER_LEN
        .checked_add(json_len)
        .ok_or("JSON length overflowed usize")?;

    if json_end > data.len() {
        return Err(format!(
            "Data length error: required {} bytes, but data len is {}",
            json_end,
            data.len()
        ).into());
    }

    let json_str = std::str::from_utf8(&data[HEADER_LEN..json_end])?;
    Ok(json_str)
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
    println!("entity create...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);
    let (tpl_pubkey, _tpl_bump) = find_pda(&[b"entity_tpl"], program_id);
    let (protocol_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut main_accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(body_pubkey, false),
        AccountMeta::new_readonly(tpl_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let allParentNames = parse_enity_full_name(&name)?;
    let allNamesLength = allParentNames.len();
    let parent_name = allParentNames[allNamesLength - 2].clone();
    let eco_name = allParentNames[0].clone();

    let parent_hash = hash_name(&parent_name);
    let (parent_meta_pda, _) = find_pda(&[b"meta", &parent_hash], program_id);
    let parent_meta_account = rpc_client.get_account(&parent_meta_pda).await
        .map_err(|_| format!("Meta config account does not exist: {}", parent_meta_pda))?;    
    let parent_json_str = extract_json_str(&parent_meta_account.data)?;
    let parent_config: MetaConfig = serde_json::from_str(parent_json_str)?;    
    let parent_fee_receiver_pubkey = Pubkey::from_str(&parent_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", parent_config.fee_receiver))?;
    main_accounts.push(AccountMeta::new(protocol_pubkey, false));
    main_accounts.push(AccountMeta::new(parent_fee_receiver_pubkey, false));

    let protocol_meta_account = rpc_client.get_account(&protocol_pubkey).await
        .map_err(|_| format!("protocol config account does not exist: {}", protocol_pubkey))?; 
    let protocol_json_str = extract_json_str(&protocol_meta_account.data)?;
    let protocol_config: TreasuryConfig = serde_json::from_str(protocol_json_str)?;
    let protocol_fee_receiver_pubkey = Pubkey::from_str(&protocol_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", protocol_config.fee_receiver))?;
    main_accounts.push(AccountMeta::new(protocol_fee_receiver_pubkey, false));

    let eco_hash = hash_name(&eco_name);
    let (eco_meta_pda, _) = find_pda(&[b"meta", &eco_hash], program_id);
    let eco_meta_account = rpc_client.get_account(&eco_meta_pda).await
        .map_err(|_| format!("Eco meta config account does not exist: {}", eco_meta_pda))?;
    let eco_json_str = extract_json_str(&eco_meta_account.data)?;
    let eco_config: MetaConfig = serde_json::from_str(eco_json_str)?;
    let pay_token_str = eco_config.pay_token.ok_or("wrong paytoken")?;

    let mut all_remaining_accounts = vec![];

    all_remaining_accounts.push(AccountMeta::new(eco_meta_pda, false));
    all_remaining_accounts.push(AccountMeta::new(parent_meta_pda, false));

    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(&pay_token_str)
            .map_err(|_| format!("invalid mint pubkey: {}", pay_token_str))?;

        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            payer_pbk, 
            &mint_pubkey,
            &token2022_program_pbk
        );

        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &parent_fee_receiver_pubkey,
            &mint_pubkey,
            &token2022_program_pbk
        );

        all_remaining_accounts.push(AccountMeta::new(to_ata, false)); // to_ata
        all_remaining_accounts.push(AccountMeta::new(from_ata, false)); 
        all_remaining_accounts.push(AccountMeta::new(*payer_pbk, true));           // authority
        all_remaining_accounts.push(AccountMeta::new(mint_pubkey, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false)); // token_program
    }    

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_create"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    data.extend_from_slice(meta.as_bytes());

    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(body.as_bytes());

    data.push(years);

    let mut full_account_keys = main_accounts.clone();
    full_account_keys.extend(all_remaining_accounts.clone());

    let instruction = Instruction {
        program_id: *program_id,
        accounts: full_account_keys.clone(),
        data,
    };
    let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(400_000);

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[compute_budget_instruction, instruction], Some(payer_pbk));
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
         
    println!("entity created. hash: {}", signature);
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
    println!("entity update meta...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (tpl_pubkey, _tpl_bump) = find_pda(&[b"entity_tpl"], program_id);
    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new_readonly(tpl_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_meta_update"));

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
         
    println!("entity update meta finished. hash: {}", signature);
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
    println!("entity update body...");
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
    data.extend_from_slice(&instruction_discriminator("entity_body_update"));

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
         
    println!("entity update body finished. hash: {}", signature);
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
    println!("entity ownership renew...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);  
    let (protocol_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut main_accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let allParentNames = parse_enity_full_name(&name)?;
    let allNamesLength = allParentNames.len();
    let parent_name = allParentNames[allNamesLength - 2].clone();
    let eco_name = allParentNames[0].clone();

    let parent_hash = hash_name(&parent_name);
    let (parent_meta_pda, _) = find_pda(&[b"meta", &parent_hash], program_id);
    let parent_meta_account = rpc_client.get_account(&parent_meta_pda).await
        .map_err(|_| format!("Meta config account does not exist: {}", parent_meta_pda))?;    
    let parent_json_str = extract_json_str(&parent_meta_account.data)?;
    let parent_config: MetaConfig = serde_json::from_str(parent_json_str)?;    
    let parent_fee_receiver_pubkey = Pubkey::from_str(&parent_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", parent_config.fee_receiver))?;
    main_accounts.push(AccountMeta::new(protocol_pubkey, false));
    main_accounts.push(AccountMeta::new(parent_fee_receiver_pubkey, false));

    let protocol_meta_account = rpc_client.get_account(&protocol_pubkey).await
        .map_err(|_| format!("protocol config account does not exist: {}", protocol_pubkey))?; 
    let protocol_json_str = extract_json_str(&protocol_meta_account.data)?;
    let protocol_config: TreasuryConfig = serde_json::from_str(protocol_json_str)?;
    let protocol_fee_receiver_pubkey = Pubkey::from_str(&protocol_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", protocol_config.fee_receiver))?;    
    main_accounts.push(AccountMeta::new(protocol_fee_receiver_pubkey, false));

    let eco_hash = hash_name(&eco_name);
    let (eco_meta_pda, _) = find_pda(&[b"meta", &eco_hash], program_id);
    let eco_meta_account = rpc_client.get_account(&eco_meta_pda).await
        .map_err(|_| format!("Eco meta config account does not exist: {}", eco_meta_pda))?;
    let eco_json_str = extract_json_str(&eco_meta_account.data)?;
    let eco_config: MetaConfig = serde_json::from_str(eco_json_str)?;
    let pay_token_str = eco_config.pay_token.ok_or("wrong paytoken")?;    

    let mut all_remaining_accounts = vec![];
    all_remaining_accounts.push(AccountMeta::new(eco_meta_pda, false));
    all_remaining_accounts.push(AccountMeta::new(parent_meta_pda, false));

    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(&pay_token_str)
            .map_err(|_| format!("invalid mint pubkey: {}", pay_token_str))?;

        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
                payer_pbk, 
                &mint_pubkey,
                &token2022_program_pbk);
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &parent_fee_receiver_pubkey,
            &mint_pubkey,
            &token2022_program_pbk
        );

        all_remaining_accounts.push(AccountMeta::new(to_ata, false)); // to_ata
        all_remaining_accounts.push(AccountMeta::new(from_ata, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new(*payer_pbk, true));           // authority
        all_remaining_accounts.push(AccountMeta::new(mint_pubkey, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false)); // token_program
    }

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_owner_renew"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.push(years); 

    let mut full_account_keys = main_accounts.clone();
    full_account_keys.extend(all_remaining_accounts.clone());

    let instruction = Instruction {
        program_id: *program_id,
        accounts: full_account_keys.clone(),
        data,
    };
    let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(400_000);

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[compute_budget_instruction, instruction], Some(payer_pbk));
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
         
    println!("entity ownership renew finished. hash: {}", signature);
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
    println!("entity ownership transfer...");
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
    data.extend_from_slice(&instruction_discriminator("entity_owner_transfer"));

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
         
    println!("entity ownership transfer finished. hash: {}", signature);
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
    let rpc_client: &_ = &config.rpc_client;
    println!("entity sale ask...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_sale_ask"));

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
         
    println!("entity sale ask finished. hash: {}", signature);
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
    println!("entity sale bid...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (protocol_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);
    let (body_pubkey, _body_bump) = find_pda(&[b"body", &name_hash], program_id);   
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut main_accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(body_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let allParentNames = parse_enity_full_name(&name)?;
    let allNamesLength = allParentNames.len();
    let parent_name = allParentNames[allNamesLength - 2].clone();
    let eco_name = allParentNames[0].clone();

    let protocol_meta_account = rpc_client.get_account(&protocol_pubkey).await
        .map_err(|_| format!("protocol config account does not exist: {}", protocol_pubkey))?; 
    let protocol_json_str = extract_json_str(&protocol_meta_account.data)?;
    let protocol_config: TreasuryConfig = serde_json::from_str(protocol_json_str)?;
    let protocol_fee_receiver_pubkey = Pubkey::from_str(&protocol_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", protocol_config.fee_receiver))?;
    main_accounts.push(AccountMeta::new(protocol_pubkey, false));
    main_accounts.push(AccountMeta::new(protocol_fee_receiver_pubkey, false));

    let meta_account = rpc_client.get_account(&meta_pubkey).await
        .map_err(|_| format!("Meta config account does not exist: {}", meta_pubkey))?;    
    let json_str = extract_json_str(&meta_account.data)?;
    let config: MetaConfig = serde_json::from_str(json_str)?;    
    let fee_receiver_pubkey = Pubkey::from_str(&config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", config.fee_receiver))?;    
    main_accounts.push(AccountMeta::new(fee_receiver_pubkey, false));

    let eco_hash = hash_name(&eco_name);
    let (eco_meta_pda, _) = find_pda(&[b"meta", &eco_hash], program_id);
    let eco_meta_account = rpc_client.get_account(&eco_meta_pda).await
        .map_err(|_| format!("Eco meta config account does not exist: {}", eco_meta_pda))?;
    let eco_json_str = extract_json_str(&eco_meta_account.data)?;
    let eco_config: MetaConfig = serde_json::from_str(eco_json_str)?;
    let pay_token_str = eco_config.pay_token.ok_or("wrong paytoken")?;

    let mut all_remaining_accounts = vec![];
    all_remaining_accounts.push(AccountMeta::new(eco_meta_pda, false));

    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(&pay_token_str)?;
        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
                payer_pbk, 
                &mint_pubkey,
                &token2022_program_pbk);
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fee_receiver_pubkey,
            &mint_pubkey,
            &token2022_program_pbk
        );
        all_remaining_accounts.push(AccountMeta::new(to_ata, false)); // to_ata
        all_remaining_accounts.push(AccountMeta::new(from_ata, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new(*payer_pbk, true));           // authority
        all_remaining_accounts.push(AccountMeta::new(mint_pubkey, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false)); // token_program
    }

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_sale_bid"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    let mut full_account_keys = main_accounts.clone();
    full_account_keys.extend(all_remaining_accounts.clone());

    let instruction = Instruction {
        program_id: *program_id,
        accounts: full_account_keys.clone(),
        data,
    };
    let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(400_000);

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[compute_budget_instruction, instruction], Some(payer_pbk));
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
         
    println!("entity sale bid finished. hash: {}", signature);
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
    println!("entity rent ask...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);

    let accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_rent_ask"));

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
         
    println!("entity rent ask finished. hash: {}", signature);
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
    println!("entity rent bid...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (rent_pubkey, _rent_bump) = find_pda(&[b"rent", &name_hash], program_id);
    let (protocol_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut main_accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(rent_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];
    let allParentNames = parse_enity_full_name(&name)?;
    let allNamesLength = allParentNames.len();
    let parent_name = allParentNames[allNamesLength - 2].clone();
    let eco_name = allParentNames[0].clone();

    let meta_account = rpc_client.get_account(&meta_pubkey).await
        .map_err(|_| format!("Meta config account does not exist: {}", meta_pubkey))?;    
    let json_str = extract_json_str(&meta_account.data)?;
    let config: MetaConfig = serde_json::from_str(json_str)?;    
    let fee_receiver_pubkey = Pubkey::from_str(&config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", config.fee_receiver))?;    
	
    main_accounts.push(AccountMeta::new(protocol_pubkey, false));
    main_accounts.push(AccountMeta::new(fee_receiver_pubkey, false));

    let protocol_meta_account = rpc_client.get_account(&protocol_pubkey).await
        .map_err(|_| format!("protocol config account does not exist: {}", protocol_pubkey))?; 
    let protocol_json_str = extract_json_str(&protocol_meta_account.data)?;
    let protocol_config: TreasuryConfig = serde_json::from_str(protocol_json_str)?;
    let protocol_fee_receiver_pubkey = Pubkey::from_str(&protocol_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", protocol_config.fee_receiver))?;
    main_accounts.push(AccountMeta::new(protocol_fee_receiver_pubkey, false));

    let eco_hash = hash_name(&eco_name);
    let (eco_meta_pda, _) = find_pda(&[b"meta", &eco_hash], program_id);
    let eco_meta_account = rpc_client.get_account(&eco_meta_pda).await
        .map_err(|_| format!("Eco meta config account does not exist: {}", eco_meta_pda))?;
    let eco_json_str = extract_json_str(&eco_meta_account.data)?;
    let eco_config: MetaConfig = serde_json::from_str(eco_json_str)?;
    let pay_token_str = eco_config.pay_token.ok_or("wrong paytoken")?;

    let mut all_remaining_accounts = vec![];
    all_remaining_accounts.push(AccountMeta::new(eco_meta_pda, false));

    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(&pay_token_str)?;
        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
                payer_pbk, 
                &mint_pubkey,
                &token2022_program_pbk);
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fee_receiver_pubkey,
            &mint_pubkey,
            &token2022_program_pbk
        );

        all_remaining_accounts.push(AccountMeta::new(to_ata, false)); // to_ata
        all_remaining_accounts.push(AccountMeta::new(from_ata, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new(*payer_pbk, true));           // authority
        all_remaining_accounts.push(AccountMeta::new(mint_pubkey, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false)); // token_program
    }

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_rent_bid"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&days.to_le_bytes()); //u32

    let mut full_account_keys = main_accounts.clone();
    full_account_keys.extend(all_remaining_accounts.clone());

    let instruction = Instruction {
        program_id: *program_id,
        accounts: full_account_keys.clone(),
        data,
    };
    let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(400_000);

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[compute_budget_instruction, instruction], Some(payer_pbk));
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
         
    println!("entity rent bid finished. hash: {}", signature);
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
    println!("entity update rent...");
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
    data.extend_from_slice(&instruction_discriminator("entity_rent_info_update"));

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
         
    println!("entity update rent finished. hash: {}", signature);
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
    println!("entity usership renew...");
    let name_hash = hash_name(&name);
    let (header_pubkey, _header_bump) = find_pda(&[b"name", &name_hash], program_id);
    let (meta_pubkey, _meta_bump) = find_pda(&[b"meta", &name_hash], program_id);
    let (protocol_pubkey, _treasury_bump) = find_pda(&[b"treasury_config"], program_id);
    let token2022_program_pbk = Pubkey::from_str("Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c").unwrap();

    let mut main_accounts = vec![
        AccountMeta::new(header_pubkey, false),
        AccountMeta::new(meta_pubkey, false),
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];
    let allParentNames = parse_enity_full_name(&name)?;
    let allNamesLength = allParentNames.len();
    let parent_name = allParentNames[allNamesLength - 2].clone();
    let eco_name = allParentNames[0].clone();

    let meta_account = rpc_client.get_account(&meta_pubkey).await
        .map_err(|_| format!("Meta config account does not exist: {}", meta_pubkey))?;    
    let json_str = extract_json_str(&meta_account.data)?;
    let config: MetaConfig = serde_json::from_str(json_str)?;    
    let fee_receiver_pubkey = Pubkey::from_str(&config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", config.fee_receiver))?;        

    let protocol_meta_account = rpc_client.get_account(&protocol_pubkey).await
        .map_err(|_| format!("protocol config account does not exist: {}", protocol_pubkey))?; 
    let protocol_json_str = extract_json_str(&protocol_meta_account.data)?;
    let protocol_config: TreasuryConfig = serde_json::from_str(protocol_json_str)?;
    let protocol_fee_receiver_pubkey = Pubkey::from_str(&protocol_config.fee_receiver)
        .map_err(|_| format!("invalid fee receiver pubkey: {}", protocol_config.fee_receiver))?;
    main_accounts.push(AccountMeta::new(protocol_pubkey, false));
    main_accounts.push(AccountMeta::new(protocol_fee_receiver_pubkey, false));
    main_accounts.push(AccountMeta::new(fee_receiver_pubkey, false));    

    let eco_hash = hash_name(&eco_name);
    let (eco_meta_pda, _) = find_pda(&[b"meta", &eco_hash], program_id);
    let eco_meta_account = rpc_client.get_account(&eco_meta_pda).await
        .map_err(|_| format!("Eco meta config account does not exist: {}", eco_meta_pda))?;
    let eco_json_str = extract_json_str(&eco_meta_account.data)?;
    let eco_config: MetaConfig = serde_json::from_str(eco_json_str)?;
    let pay_token_str = eco_config.pay_token.ok_or("wrong paytoken")?;

    let mut all_remaining_accounts = vec![];
    all_remaining_accounts.push(AccountMeta::new(eco_meta_pda, false));

    if pay_token_str != "11111111111111111111111111111111111111111111" {
        let mint_pubkey = Pubkey::from_str(&pay_token_str)?;
        let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
                payer_pbk, 
                &mint_pubkey,
                &token2022_program_pbk);
        let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fee_receiver_pubkey,
            &mint_pubkey,
            &token2022_program_pbk
        );
        all_remaining_accounts.push(AccountMeta::new(to_ata, false)); // to_ata
        all_remaining_accounts.push(AccountMeta::new(from_ata, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new(*payer_pbk, true));           // authority
        all_remaining_accounts.push(AccountMeta::new(mint_pubkey, false));             // mint        
        all_remaining_accounts.push(AccountMeta::new_readonly(token2022_program_pbk, false)); // token_program
    }

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("entity_rent_renew"));

    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    data.extend_from_slice(&days.to_le_bytes()); //u32

    let mut full_account_keys = main_accounts.clone();
    full_account_keys.extend(all_remaining_accounts.clone());

    let instruction = Instruction {
        program_id: *program_id,
        accounts: full_account_keys.clone(),
        data,
    };
    let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(400_000);

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[compute_budget_instruction, instruction], Some(payer_pbk));
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
         
    println!("entity renew usership finished. hash: {}", signature);
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
    println!("entity usership transfer...");
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
    data.extend_from_slice(&instruction_discriminator("entity_rent_transfer"));

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
         
    println!("entity usership transfer finished. hash: {}", signature);
    Ok(())
}
