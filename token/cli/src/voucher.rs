/// The `entity` subcommand
use {
    crate::{clap_app::Error, command::CommandResult, config::Config}, clap::{value_t_or_exit, ArgMatches}, futures::future::ok, solana_clap_utils::input_parsers::pubkey_of_signer, 
    solana_client::{
        nonblocking::rpc_client::RpcClient, rpc_client::RpcClient as BlockingRpcClient,
        tpu_client::{TpuClient, TpuClientConfig},
    }, solana_remote_wallet::remote_wallet::RemoteWalletManager, 
    solana_sdk::{
        message::Message, native_token::{lamports_to_sol, Sol}, program_pack::Pack, pubkey::Pubkey, 
        signature::{Signature, Signer}, signer, system_instruction, instruction::{AccountMeta, Instruction}, transaction::Transaction,
        compute_budget::ComputeBudgetInstruction,
    }, 
    spl_associated_token_account::*, spl_token_2022::{
        extension::StateWithExtensions,
        instruction,
        state::{Account, Mint},
    }, 
    std::{rc::Rc, sync::Arc, time::Instant, str::FromStr}, 
    crate::utils::{find_pda, VoucherTreasuryConfig as TreasuryConfig, instruction_discriminator, 
        VOUCHER_PROGRAM_ID,parse_voucher_detail,VoucherResult,keypair_from_base58,export_vouchers_to_excel,
        get_mint_decimals,get_or_create_token_ata,TOKEN2022_PROGRAM_ID,format_timestamp,parse_vault,
    }, 
    solana_program::{pubkey::Pubkey as SolPubkey}, 
    serde_json::Value,
    spl_associated_token_account::{ error::AssociatedTokenAccountError, get_associated_token_address_with_program_id,
        instruction::{create_associated_token_account,},
    },
    bs58,
    rust_xlsxwriter::{Format, Workbook},
    chrono::Local,
};

pub(crate) async fn voucher_process_command(
    matches: &ArgMatches<'_>,
    config: &Config<'_>,
    mut signers: Vec<Arc<dyn Signer>>,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
) -> CommandResult {
    assert!(!config.sign_only);

    match matches.subcommand() {
        ("create", Some(arg_matches)) => {
            let mint = value_t_or_exit!(arg_matches, "mint", String);
            let quota = value_t_or_exit!(arg_matches, "quota", f64);
            let count = value_t_or_exit!(arg_matches, "count", u64);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(VOUCHER_PROGRAM_ID).unwrap();
            command_create(config, signers, mint, quota, count, &owner, &program_id).await?;
        }
        ("redeem", Some(arg_matches)) => {
            let code = value_t_or_exit!(arg_matches, "code", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(VOUCHER_PROGRAM_ID).unwrap();
            command_redeem(config, signers, code, &owner, &program_id).await?;
        }
        ("withdraw", Some(arg_matches)) => {
            let mint = value_t_or_exit!(arg_matches, "mint", String);
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(VOUCHER_PROGRAM_ID).unwrap();
            command_withdraw(config, signers, mint, amount, &owner, &program_id).await?;
        }
        ("info", Some(arg_matches)) => {
            let code = value_t_or_exit!(arg_matches, "code", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(VOUCHER_PROGRAM_ID).unwrap();
            command_info(config, signers, code, &owner, &program_id).await?;
        }
        ("vault", Some(arg_matches)) => {
            let mint = value_t_or_exit!(arg_matches, "mint", String);
            let (owner_signer, owner) =
                config.signer_or_default_ons(arg_matches, "owner", wallet_manager);
            signers.push(owner_signer);
            let program_id = SolPubkey::from_str(VOUCHER_PROGRAM_ID).unwrap();
            command_vault(config, signers, mint, &owner, &program_id).await?;
        }
        _ => unreachable!(),
    }

    Ok("".to_string())
}

async fn command_create(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    mint: String,
    quota: f64,
    count: u64,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    let token2022_program_pbk = Pubkey::from_str(TOKEN2022_PROGRAM_ID).unwrap();
    if quota < 0.0 {
        return Err(format!("Quota must be >= 0").into());
    }  
    if count < 1 {
        return Err(format!("Count must be >= 1").into());
    }
    let mint_pubkey = Pubkey::from_str(&mint)
            .map_err(|_| {
                format!(
                    "Invalid mint address: {}",
                    mint
                )
            })?;
    println!("==========================================");
    println!("Voucher Create");
    println!("==========================================");
    println!("Operator : {}", payer_pbk);
    println!("Mint     : {}", mint_pubkey);
    println!("Quota    : {}", quota);
    println!("Count    : {}", count);
    let decimals = get_mint_decimals(rpc_client, &mint_pubkey)
        .await
        .map_err(|e| {
            format!(
                "Failed to get mint decimals: {}",
                e
            )
        })?;

    //println!("Mint decimals: {}", decimals);
    let quota_amount = spl_token::ui_amount_to_amount(quota, decimals);
    //println!("Quota raw amount: {}", quota_amount);
    let (treasury_pubkey, _) = find_pda(&[b"treasury_config"], program_id);
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
    //println!("fee_receiver_pubkey: {}", &treasury_config.fees);

    let operator_token_account = get_or_create_token_ata(
            rpc_client,
            payer_pbk,
            &signers,
            payer_pbk,
            &mint_pubkey,
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to get/create operator ATA: {}",
                e
            )
        })?;
    //println!("Operator ATA: {}", operator_token_account);
    let (vault_pubkey, vault_bump) = Pubkey::find_program_address(
            &[
                b"vault",
                payer_pbk.as_ref(),
                mint_pubkey.as_ref(),
            ],
            program_id,
        );

    //println!("Vault PDA: {} bump: {}", vault_pubkey, vault_bump);
    let vault_token_account = get_or_create_token_ata(
            rpc_client,
            payer_pbk,
            &signers,
            &vault_pubkey,
            &mint_pubkey,
        )
        .await
        .map_err(|e| {
            format!("Failed to get/create vault ATA: {}", e)
        })?;

    //println!("Vault ATA: {}", vault_token_account);
    let mut results: Vec<VoucherResult> = Vec::with_capacity(count as usize);
    for index in 0..count {
        println!();
        println!("========== [{}/{}] ==========", index + 1, count);
        let voucher_keypair = solana_sdk::signature::Keypair::new();
        let public_key = voucher_keypair.pubkey();
        let redeem_code = bs58::encode(voucher_keypair.to_bytes()).into_string();
        println!("Public_Key : {}", public_key);
        println!("Redeem_Code: {}", redeem_code);
        let (detail_pubkey, detail_bump) =
            Pubkey::find_program_address(
                &[b"vo", public_key.as_ref()],
                program_id,
            );

        println!("Detail PDA : {}", detail_pubkey);
        let accounts = vec![
            AccountMeta::new(detail_pubkey, false),
            AccountMeta::new(*payer_pbk, true),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(treasury_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(fee_receiver_pubkey, false),
            AccountMeta::new_readonly(mint_pubkey, false),
            AccountMeta::new_readonly(token2022_program_pbk, false),
            AccountMeta::new(operator_token_account, false),
            AccountMeta::new(vault_token_account, false),
        ];

        let mut data = Vec::with_capacity(8 + 32 + 8);

        data.extend_from_slice(&instruction_discriminator("voucher_create"));
        data.extend_from_slice(public_key.as_ref());
        data.extend_from_slice(&quota_amount.to_le_bytes());

        let instruction = Instruction {
            program_id: *program_id,
            accounts,
            data,
        };

        let blockhash = rpc_client.get_latest_blockhash()
                .await
                .map_err(|e| {
                    format!(
                        "Failed to get blockhash: {}",
                        e
                    )
                })?;
        let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(400_000);
        let mut transaction = Transaction::new_with_payer(&[compute_budget_instruction, instruction], Some(payer_pbk));
        transaction.sign(&signers, blockhash);

        let simulation = rpc_client.simulate_transaction(&transaction)
                .await
                .map_err(|e| {
                    format!(
                        "Simulation RPC failed: {}",
                        e
                    )
                })?;

        if let Some(err) = simulation.value.err{
            println!();
            println!(
                "Voucher {} simulation failed:",
                index + 1
            );

            for log in simulation
                .value
                .logs
                .unwrap_or_default()
            {
                println!("{}", log);
            }

            return Err(
                format!(
                    "Simulation failed: {:?}",
                    err
                )
                .into()
            );
        }

        let signature = rpc_client.send_and_confirm_transaction(&transaction)
                .await
                .map_err(|e| {
                    format!(
                        "Voucher {} send failed: {}",
                        index + 1,
                        e
                    )
                })?;

        println!("SUCCESS tx : {}", signature);
        results.push(
            VoucherResult {
                index: index + 1,
                public_key,
                redeem_code,
                signature,
            }
        );
    }
    let filename = format!("voucher_{}.xlsx", Local::now().format("%Y%m%d_%H%M%S"));
    export_vouchers_to_excel(&filename, &results, decimals,)
        .map_err(|e| {
            format!(
                "Failed to export Excel: {}",
                e
            )
        })?;

    println!();
    println!("==========================================");
    println!("Voucher creation completed.");
    println!("Mint             : {}", mint_pubkey);
    println!("Decimals         : {}", decimals);
    println!("Quota each       : {}", quota);
    println!("Raw quota each   : {}", quota_amount);
    println!("Count            : {}", results.len());
    println!("Vault            : {}", vault_pubkey);
    println!("Vault ATA        : {}", vault_token_account);
    println!("Excel            : {}", filename);
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("⚠️ IMPORTANT: Critical voucher data. Please save this file first!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

    println!(
        "=========================================="
    );    
    Ok(())
}

async fn command_redeem(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    code: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    let token2022_program_pbk = Pubkey::from_str(TOKEN2022_PROGRAM_ID).unwrap();
    let voucher_keypair = keypair_from_base58(&code)?;
    let voucher_pubkey = voucher_keypair.pubkey();
    println!("==========================================");
    println!("Voucher Redeem");
    println!("==========================================");
    let (detail_pubkey, _) =
        Pubkey::find_program_address(
            &[b"vo", voucher_pubkey.as_ref()],
            program_id,
        );
    let detail_account = rpc_client.get_account(&detail_pubkey)
            .await
            .map_err(|e| {
                format!(
                    "Voucher does not exist. \
                     Detail {} not found: {}",
                    detail_pubkey,
                    e
                )
            })?;

    let detail = parse_voucher_detail(&detail_account.data)?;
    let (vault_pubkey, _) =
        Pubkey::find_program_address(
            &[
                b"vault",
                detail.creator.as_ref(),
                detail.mint.as_ref(),
            ],
            program_id,
        );
    let user_token_account = get_or_create_token_ata(
            rpc_client,
            payer_pbk,
            &signers,
            payer_pbk,
            &detail.mint,
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to get/create user ATA: {}",
                e
            )
        })?;

    let vault_token_account = get_or_create_token_ata(
            rpc_client,
            payer_pbk,
            &signers,
            &vault_pubkey,
            &detail.mint,
        )
        .await
        .map_err(|e| {
            format!("Failed to get/create vault ATA: {}", e)
        })?;


    let accounts = vec![
        AccountMeta::new(*payer_pbk, true), // operator
        AccountMeta::new_readonly(voucher_pubkey, true),
        AccountMeta::new_readonly(detail.mint, false),
        AccountMeta::new(vault_pubkey, false),
        AccountMeta::new(detail_pubkey, false),
        AccountMeta::new(vault_token_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(token2022_program_pbk, false),
    ];

    let mut data = Vec::new();
    data.extend_from_slice(&instruction_discriminator("voucher_redeem"));

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));

    let mut all_signers: Vec<&dyn Signer> = Vec::new();

    for signer in signers.iter(){
        all_signers.push(
            signer.as_ref()
        );
    }
    all_signers.push(
        &voucher_keypair
    );
    transaction.sign(&all_signers, blockhash);

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
         
    println!("Voucher redeemed successfully!");

    println!("Public-key       : {}", voucher_pubkey);
    println!("User             : {}", payer_pbk);
    println!("Mint             : {}", detail.mint);
    println!("Raw Quota        : {}", detail.quota);
    println!("Vault            : {}", vault_pubkey);
    println!("Vault ATA        : {}", vault_token_account);
    println!("User ATA         : {}", user_token_account);
    println!("Transaction      : {}", signature);
    println!("==========================================");
    Ok(())
}

async fn command_withdraw(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    mint: String,
    amount: f64,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    let token2022_program_pbk = Pubkey::from_str(TOKEN2022_PROGRAM_ID).unwrap();
    if amount < 0.0 {
        return Err(format!("Amount must be >= 0").into());
    }  
    let mint_pubkey = Pubkey::from_str(&mint)
            .map_err(|_| {
                format!(
                    "Invalid mint address: {}",
                    mint
                )
            })?;
    println!("==========================================");
    println!("Voucher Withdraw");
    println!("==========================================");
    println!("Operator : {}", payer_pbk);
    println!("Mint     : {}", mint_pubkey);
    println!("Amount   : {}", amount);
    let decimals = get_mint_decimals(rpc_client, &mint_pubkey)
        .await
        .map_err(|e| {
            format!(
                "Failed to get mint decimals: {}",
                e
            )
        })?;

    let raw_amount = spl_token::ui_amount_to_amount(amount, decimals);
    let operator_token_account = get_or_create_token_ata(
            rpc_client,
            payer_pbk,
            &signers,
            payer_pbk,
            &mint_pubkey,
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to get/create operator ATA: {}",
                e
            )
        })?;
    //println!("Operator ATA: {}", operator_token_account);
    let (vault_pubkey, vault_bump) = Pubkey::find_program_address(
            &[
                b"vault",
                payer_pbk.as_ref(),
                mint_pubkey.as_ref(),
            ],
            program_id,
        );

    //println!("Vault PDA: {} bump: {}", vault_pubkey, vault_bump);
    let vault_token_account = get_or_create_token_ata(
            rpc_client,
            payer_pbk,
            &signers,
            &vault_pubkey,
            &mint_pubkey,
        )
        .await
        .map_err(|e| {
            format!("Failed to get/create vault ATA: {}", e)
        })?;

    println!();
    let accounts = vec![
        AccountMeta::new(*payer_pbk, true),        
        AccountMeta::new_readonly(mint_pubkey, false),
        AccountMeta::new(vault_pubkey, false),
        AccountMeta::new(vault_token_account, false),        
        AccountMeta::new(operator_token_account, false),        
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(token2022_program_pbk, false),
    ];

    let mut data = Vec::with_capacity(8 + 8);
    data.extend_from_slice(&instruction_discriminator("vault_withdraw"));
    data.extend_from_slice(&raw_amount.to_le_bytes());

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data,
    };

    let blockhash = rpc_client.get_latest_blockhash()
        .await
        .map_err(|e| {
            format!(
                "Failed to get blockhash: {}",
                e
            )
        })?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(payer_pbk));
    transaction.sign(&signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction)
        .await
        .map_err(|e| {
            format!(
                "Simulation RPC failed: {}",
                e
            )
        })?;

    if let Some(err) = simulation.value.err{
        println!();
        println!(
            "Voucher simulation failed:"
        );

        for log in simulation
            .value
            .logs
            .unwrap_or_default()
        {
            println!("{}", log);
        }

        return Err(
            format!(
                "Simulation failed: {:?}",
                err
            )
            .into()
        );
    }

    let signature = rpc_client.send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| {
                format!(
                    "Voucher send failed: {}",
                    e
                )
            })?;

    println!("SUCCESS tx : {}", signature);
    println!();
    println!("==========================================");
    println!("Voucher withdraw completed.");
    println!("Mint             : {}", mint_pubkey);
    println!("Decimals         : {}", decimals);
    println!("Amount           : {}", amount);
    println!("Raw amount       : {}", raw_amount);
    println!("Vault            : {}", vault_pubkey);
    println!("Vault ATA        : {}", vault_token_account);
    println!("==========================================");    
    Ok(())
}

async fn command_info(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    code: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    let voucher_pubkey = Pubkey::from_str(&code)
            .map_err(|_| {
                format!(
                    "Invalid voucher public key: {}",
                    code
                )
            })?;
    let (detail_pubkey, _) =
        Pubkey::find_program_address(
            &[b"vo", voucher_pubkey.as_ref()],
            program_id,
        );
    let detail_account = rpc_client.get_account(&detail_pubkey)
            .await
            .map_err(|e| {
                format!(
                    "Voucher does not exist. \
                     Detail {} not found: {}",
                    detail_pubkey,
                    e
                )
            })?;

    let detail = parse_voucher_detail(&detail_account.data)?;
    let mint_pubkey = detail.mint;
    println!("==========================================");
    println!("Voucher Information");
    println!("==========================================");
    let decimals = get_mint_decimals(rpc_client, &mint_pubkey)
        .await
        .map_err(|e| {
            format!(
                "Failed to get mint decimals: {}",
                e
            )
        })?;

    let human_amount = spl_token::amount_to_ui_amount(detail.quota, decimals);
    let create_time = format_timestamp(detail.create_time);
    let redeem_time = format_timestamp(detail.redeem_time);
    let redeemer = if detail.redeemer == Pubkey::default() {
        String::new()
    } else {
        detail.redeemer.to_string()
    };
    println!("Public key       : {}", voucher_pubkey);
    println!("Mint             : {}", mint_pubkey);
    println!("Decimals         : {}", decimals);    
    println!("Quota            : {}", human_amount);
    println!("Raw quota        : {}", detail.quota);
    println!("Owner            : {}", detail.creator);
    println!("Create time      : {}", create_time);
    println!("Redeemer         : {}", redeemer);
    println!("Redeem time      : {}", redeem_time);
    println!("==========================================");    
    Ok(())
}

async fn command_vault(
    config: &Config<'_>,
    signers: Vec<Arc<dyn Signer>>,
    mint: String,
    payer_pbk: &Pubkey,
    program_id: &Pubkey,
) -> Result<(), Error> {
    let rpc_client = &config.rpc_client;
    let mint_pubkey = Pubkey::from_str(&mint)
            .map_err(|_| {
                format!(
                    "Invalid mint address: {}",
                    mint
                )
            })?;
    println!("==========================================");
    println!("Voucher Vault");
    println!("==========================================");
    let decimals = get_mint_decimals(rpc_client, &mint_pubkey)
        .await
        .map_err(|e| {
            format!(
                "Failed to get mint decimals: {}",
                e
            )
        })?;
    let (vault_pubkey, _) = Pubkey::find_program_address(
            &[
                b"vault",
                payer_pbk.as_ref(),
                mint_pubkey.as_ref(),
            ],
            program_id,
        );
    let account = rpc_client.get_account(&vault_pubkey)
        .await
        .map_err(|e| {
            format!(
                "Vault does not exist. operator={}, mint={}, vault={}, error={}",
                payer_pbk,
                mint_pubkey,
                vault_pubkey,
                e
            )
        })?;
    let vault = parse_vault(&account.data)?;
    let human_balance = spl_token::amount_to_ui_amount(vault.balance, decimals);
    let human_deposited = spl_token::amount_to_ui_amount(vault.total_deposited, decimals);
    let human_redeemed = spl_token::amount_to_ui_amount(vault.total_redeemed, decimals);
    let human_withdreaw = spl_token::amount_to_ui_amount(vault.total_withdrew, decimals);
    println!("Operator         : {}", payer_pbk);
    println!("Mint             : {}", mint_pubkey);
    println!("Decimals         : {}", decimals);
    println!("Vault            : {}", vault_pubkey);
    println!("Raw balance      : {}", human_balance);
    println!("balance          : {}", vault.balance);
    println!("Total deposited  : {}", human_deposited);
    println!("Raw deposited    : {}", vault.total_deposited);
    println!("Total redeemed   : {}", human_redeemed);
    println!("Raw redeemed     : {}", vault.total_redeemed);
    println!("Total withdrew   : {}", human_withdreaw);
    println!("Raw withdrew     : {}", vault.total_withdrew);
    println!("==========================================");  
    Ok(())
}