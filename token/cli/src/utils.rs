use {
    sha2::{Digest, Sha256},
    solana_sdk::{
        pubkey::Pubkey, signature::{Signature, Signer}, transaction::Transaction,
    },
    serde_json::Value,
    crate::{clap_app::Error},
    solana_client::{ nonblocking::rpc_client::RpcClient,},
    std::{rc::Rc, sync::Arc, time::{Instant, Duration}, str::FromStr},
    rust_xlsxwriter::{Format, Workbook},
    chrono::{Local, TimeZone, Utc},
    spl_associated_token_account::{ error::AssociatedTokenAccountError, get_associated_token_address_with_program_id,
        instruction::{create_associated_token_account,},
    },
    tokio::time::sleep,
};
pub const ONS_PROGRAM_ID: &str = "on6LJ2wZa2jRAdnouvPtkAxLtZfVr9y8J7dMLgeDWLg";
pub const ONS_API_URL: &str = "http://192.168.204.128:5056";
pub const VOUCHER_PROGRAM_ID: &str = "votV1qo18w3JMKX8wUAmgvdAy2dDXVc9cLUy6x5XwsQ";
pub const TOKEN2022_PROGRAM_ID: &str = "Token9ADbPtdFC3PjxaohBLGw2pgZwofdcbj6Lyaw6c";

pub fn hash_name(name: &str) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut result = [0u8; 16];
    result.copy_from_slice(&hash[..16]);
    result
}

pub fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

#[derive(serde::Deserialize)]
pub struct TreasuryConfig {
    pub fee_receiver: String,
    pub fee_shortname: u64,
    pub fee_longname: u64,
}

#[derive(serde::Deserialize)]
pub struct VoucherTreasuryConfig {
    pub fee_receiver: String,
    pub fees: u64,
}

#[derive(serde::Deserialize, Clone)]
pub struct MetaConfig {
    pub fee_receiver: String,
    pub fee_shortname: u64,
    pub fee_longname: u64,
    pub basis_points_sell: u64,
    pub basis_points_rent: u64,
    #[serde(default)] 
    pub pay_token: Option<String>,
}

pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"global:");
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut result = [0u8; 8];
    result.copy_from_slice(&hash[..8]);
    result
}

#[derive(serde::Serialize, Debug)]
pub struct HeaderInfoJson {
    pub name: String,
    pub owner: String,
    pub owner_start: i64,
    pub owner_end: i64,
    pub sell_enabled: u8,
    pub sell_price: u64,
    pub user: String,
    pub user_start: i64,
    pub user_end: i64,
    pub rent_enabled: u8,
    pub rent_per_day: u64,
}

impl HeaderInfoJson {
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let mut idx = 8; // Skip 8-byte discriminator (Anchor)

        if data.len() < idx + 4 {
            return None;
        }
        let name_len = u32::from_le_bytes([data[idx], data[idx+1], data[idx+2], data[idx+3]]) as usize;
        idx += 4;

        if data.len() < idx + name_len {
            return None;
        }
        let name = String::from_utf8(data[idx..idx + name_len].to_vec()).ok()?;
        idx += name_len;

        if data.len() < idx + 32 {
            return None;
        }
        let owner = Pubkey::new_from_array((&data[idx..idx + 32]).try_into().ok()?);
        idx += 32;

        let owner_start = i64::from_le_bytes(data[idx..idx+8].try_into().ok()?); idx += 8;
        let owner_end = i64::from_le_bytes(data[idx..idx+8].try_into().ok()?); idx += 8;

        let sell_enabled = data[idx]; idx += 1;
        let sell_price = u64::from_le_bytes(data[idx..idx+8].try_into().ok()?); idx += 8;

        let user = Pubkey::new_from_array((&data[idx..idx + 32]).try_into().ok()?); idx += 32;

        let user_start = i64::from_le_bytes(data[idx..idx+8].try_into().ok()?); idx += 8;
        let user_end = i64::from_le_bytes(data[idx..idx+8].try_into().ok()?); idx += 8;

        let rent_enabled = data[idx]; idx += 1;
        let rent_per_day = u64::from_le_bytes(data[idx..idx+8].try_into().ok()?);

        Some(HeaderInfoJson {
            name,
            owner: owner.to_string(),
            owner_start,
            owner_end,
            sell_enabled,
            sell_price,
            user: user.to_string(),
            user_start,
            user_end,
            rent_enabled,
            rent_per_day,
        })
    }
}

#[derive(serde::Serialize, Debug)]
pub struct AuthJsonConfigJson {
    pub authority: String,
    pub config_json: String,
}

impl AuthJsonConfigJson {
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let mut idx = 8; // Skip discriminator

        if data.len() < idx + 32 {
            return None;
        }
        let authority = Pubkey::new_from_array((&data[idx..idx + 32]).try_into().ok()?);
        idx += 32;

        if data.len() < idx + 4 {
            return None;
        }
        let json_len = u32::from_le_bytes([data[idx], data[idx+1], data[idx+2], data[idx+3]]) as usize;
        idx += 4;

        if data.len() < idx + json_len {
            return None;
        }
        let config_json = String::from_utf8(data[idx..idx + json_len].to_vec()).ok()?;

        Some(AuthJsonConfigJson {
            authority: authority.to_string(),
            config_json,
        })
    }
    pub fn deserializee(data: &[u8]) -> Option<Value> {
        let mut idx = 8; // Skip discriminator

        if data.len() < idx + 32 {
            return None;
        }
        idx += 32;

        if data.len() < idx + 4 {
            return None;
        }
        let json_len = u32::from_le_bytes([data[idx], data[idx+1], data[idx+2], data[idx+3]]) as usize;
        idx += 4;

        if data.len() < idx + json_len {
            return None;
        }
        let config_json_str = String::from_utf8(data[idx..idx + json_len].to_vec()).ok()?;
        let json_value: Value = match serde_json::from_str(&config_json_str) {
            Ok(val) => val,                   
            Err(_) => Value::String(config_json_str),
        };
        Some(json_value)
    }
}

#[derive(Debug)]
pub struct VoucherDetailCli {
    pub code: Pubkey,
    pub mint: Pubkey,
    pub quota: u64,
    pub creator: Pubkey,
    pub create_time: i64,
    pub redeem_time: i64,
    pub redeemer: Pubkey,
}

pub fn parse_voucher_detail(
    data: &[u8],
) -> Result<VoucherDetailCli, Error> {

    // discriminator 8 bytes
    const HEADER: usize = 8;

    // 8 + 32 + 32 + 8 + 32 + 8 + 8 + 32
    const SIZE: usize =
        8 + 32 + 32 + 8 + 32 + 8 + 8 + 32;

    if data.len() < SIZE {
        return Err(
            format!(
                "VoucherDetail account data too short: {} < {}",
                data.len(),
                SIZE
            )
            .into()
        );
    }

    let mut offset = HEADER;
    // --------------------------------------------------------
    // code
    // --------------------------------------------------------
    let code =
        Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| {
                    "Invalid code pubkey"
                })?
        );

    offset += 32;
    // --------------------------------------------------------
    // mint
    // --------------------------------------------------------
    let mint =
        Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| {
                    "Invalid mint pubkey"
                })?
        );

    offset += 32;
    // --------------------------------------------------------
    // quota
    // --------------------------------------------------------
    let quota =
        u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid quota"
                })?
        );

    offset += 8;
    // --------------------------------------------------------
    // creator
    // --------------------------------------------------------
    let creator =
        Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| {
                    "Invalid creator pubkey"
                })?
        );

    offset += 32;
    // --------------------------------------------------------
    // create_time
    // --------------------------------------------------------
    let create_time =
        i64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid create_time"
                })?
        );

    offset += 8;
    // --------------------------------------------------------
    // redeem_time
    // --------------------------------------------------------
    let redeem_time =
        i64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid redeem_time"
                })?
        );

    offset += 8;
    // --------------------------------------------------------
    // redeemer
    // --------------------------------------------------------
    let redeemer =
        Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| {
                    "Invalid redeemer pubkey"
                })?
        );

    Ok(
        VoucherDetailCli {
            code,
            mint,
            quota,
            creator,
            create_time,
            redeem_time,
            redeemer,
        }
    )
}

#[derive(Debug)]
pub struct VoucherResult {
    pub index: u64,
    pub public_key: Pubkey,
    pub redeem_code: String,
    pub signature: Signature,
}

pub fn keypair_from_base58(
    private_key: &str,
) -> Result<solana_sdk::signature::Keypair, Error> {

    let bytes =
        bs58::decode(
            private_key.trim()
        )
        .into_vec()
        .map_err(|e| {
            format!("Invalid redeem-code.")
        })?;

    if bytes.len() != 64 {
        return Err(
            format!(
                "Invalid redeem-code length: {}",
                bytes.len()
            )
            .into()
        );
    }

    let keypair =
        solana_sdk::signature::Keypair::from_bytes(
            &bytes
        )
        .map_err(|e| {
            format!(
                "Invalid redeem-code!"
            )
        })?;

    Ok(keypair)
}

pub async fn get_or_create_token_ata(
    rpc_client: &RpcClient,
    payer: &Pubkey,
    signers: &[Arc<dyn Signer>],
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey, Error> {
    let token2022_program_pbk = Pubkey::from_str(TOKEN2022_PROGRAM_ID).unwrap();
    let ata = get_associated_token_address_with_program_id(
            owner,
            mint,
            &token2022_program_pbk,
        );

    match rpc_client.get_account(&ata).await {
        Ok(_) => {
            //println!( "ATA already exists: {}", ata);
            return Ok(ata);
            }
        Err(_) => {
        }
    }

    //println!("ATA does not exist, creating: {}",  ata);
    let create_ata_ix = create_associated_token_account(
            payer,
            owner,
            mint,
            &token2022_program_pbk,
        );

    let blockhash = rpc_client.get_latest_blockhash().await?;
    let mut transaction = Transaction::new_with_payer(&[create_ata_ix], Some(payer));
    transaction.sign(signers, blockhash);

    let simulation = rpc_client.simulate_transaction(&transaction).await?;

    if let Some(err) = simulation.value.err{
        println!("ATA creation simulation failed:");
        for log in simulation
            .value
            .logs
            .unwrap_or_default()
        {
            println!("{}", log);
        }
        return Err(format!("ATA creation simulation failed: {:?}", err).into());
    }

    let signature = rpc_client.send_and_confirm_transaction(&transaction).await?;
    //println!("ATA created successfully. tx: {}", signature);
    for i in 0..10 {
        match rpc_client.get_account(&ata).await {
            Ok(account) => {
                //println!("Vault ATA is visible after {} attempt(s)", i + 1);
                //println!("ATA owner: {}", account.owner);
                break;
            }

            Err(_) => {
                if i == 9 {
                    return Err(
                        format!(
                            "Vault ATA {} is still not visible",
                            ata
                        )
                        .into()
                    );
                }
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }
    sleep(Duration::from_millis(1000)).await;
    Ok(ata)
}
pub async fn get_mint_decimals(rpc_client: &RpcClient, mint: &Pubkey) -> Result<u8, Error> {
    let supply = rpc_client
        .get_token_supply(mint)
        .await
        .map_err(|e| {
            format!(
                "Failed to get token supply for mint {}: {}",
                mint,
                e
            )
        })?;

    Ok(supply.decimals)
}

pub fn export_vouchers_to_excel(
    filename: &str,
    vouchers: &[VoucherResult],
    decimals: u8,
) -> Result<(), Error> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    // --------------------------------------------------------
    // Header format
    // --------------------------------------------------------
    let header_format = Format::new()
            .set_bold();

    let headers = [
        "index",
        "public_key",
        "redeem_code",
        "signature",
    ];
    // --------------------------------------------------------
    // Header
    // --------------------------------------------------------
    for (col, header) in headers.iter().enumerate(){
        worksheet.write_string_with_format(
            0,
            col as u16,
            *header,
            &header_format,
        )?;
    }

    // --------------------------------------------------------
    // Rows
    // --------------------------------------------------------
    for (index, voucher) in vouchers.iter().enumerate(){
        let row =
            (index + 1) as u32;


        // index
        worksheet.write_number(
            row,
            0,
            voucher.index as f64,
        )?;


        // code
        worksheet.write_string(
            row,
            1,
            voucher.public_key.to_string(),
        )?;

        // private key
        worksheet.write_string(
            row,
            2,
            &voucher.redeem_code,
        )?;

        // tx
        worksheet.write_string(
            row,
            3,
            voucher.signature.to_string(),
        )?;
    }

    // --------------------------------------------------------
    // Column width
    // --------------------------------------------------------
    worksheet.set_column_width(
        0,
        10.0,
    )?;

    worksheet.set_column_width(
        1,
        45.0,
    )?;

    worksheet.set_column_width(
        2,
        90.0,
    )?;

    worksheet.set_column_width(
        3,
        45.0,
    )?;

    worksheet.set_column_width(
        4,
        45.0,
    )?;

    worksheet.set_column_width(
        5,
        50.0,
    )?;

    worksheet.set_column_width(
        6,
        20.0,
    )?;

    worksheet.set_column_width(
        7,
        25.0,
    )?;

    worksheet.set_column_width(
        8,
        12.0,
    )?;

    worksheet.set_column_width(
        9,
        90.0,
    )?;
    workbook.save(filename)?;

    Ok(())
}

pub fn format_timestamp(timestamp: i64) -> String {
    if timestamp == 0 {
        return String::new();
    }

    match Utc.timestamp_opt(timestamp, 0).single() {
        Some(dt) => {
            dt.format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
        }

        None => String::new(),
    }
}
#[derive(Debug)]
pub struct VaultCli {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub balance: u64,
    pub total_deposited: u64,
    pub total_redeemed: u64,
    pub total_withdrew: u64,
    pub bump: u8,
}
pub fn parse_vault(
    data: &[u8],
) -> Result<VaultCli, Error> {

    const DISCRIMINATOR_LEN: usize = 8;

    const VAULT_DATA_LEN: usize =
        8 + 32 + 32 + 8 + 8 + 8 + 8 + 1;

    if data.len() < VAULT_DATA_LEN {
        return Err(
            format!(
                "Invalid Vault account data length: {}, expected at least {}",
                data.len(),
                VAULT_DATA_LEN
            )
            .into()
        );
    }

    let mut offset = DISCRIMINATOR_LEN;
    let owner =
        Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| {
                    "Invalid Vault.owner"
                })?
        );

    offset += 32;
    let mint =
        Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| {
                    "Invalid Vault.mint"
                })?
        );

    offset += 32;
    let balance =
        u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid Vault.balance"
                })?
        );

    offset += 8;
    let total_deposited =
        u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid Vault.total_deposited"
                })?
        );

    offset += 8;
    let total_redeemed =
        u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid Vault.total_redeemed"
                })?
        );

    offset += 8;
    let total_withdrew =
        u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| {
                    "Invalid Vault.total_withdrew"
                })?
        );

    offset += 8;
    let bump = data[offset];

    Ok(
        VaultCli {
            owner,
            mint,
            balance,
            total_deposited,
            total_redeemed,
            total_withdrew,
            bump,
        }
    )
}