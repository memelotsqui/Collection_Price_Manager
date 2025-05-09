use anchor_lang::prelude::*;
use mpl_bubblegum::accounts::TreeConfig;
use anchor_lang::system_program;

declare_id!("FV2936jpAPgHkguQeefLpMJm6hJdcmHLy2pDCNTb13Xv");

pub const BUBBLEGUM_COMPRESSION_PROGRAM_ID: Pubkey = pubkey!("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK");

#[program]
pub mod collection_price_manager {
    use super::*;


    // Fetch collection size & prices
    pub fn fetch_prices(ctx: Context<FetchPrices>) -> Result<(u16, Pubkey, Vec<u64>)> {
        let price_data = &ctx.accounts.collection_prices;
        Ok((price_data.size, price_data.payment_mint, price_data.prices.clone()))
    }

    // Update prices (only collection owner can modify)
    pub fn update_prices(ctx: Context<UpdatePrices>, new_prices: Vec<u64>) -> Result<()> {
        let expected_bump = ctx.bumps.collection_prices;
        let stored_bump = ctx.accounts.collection_prices.bump;
        let collection_address = ctx.accounts.collection_address.key();
    
        require_eq!(expected_bump, stored_bump, ErrorCode::InvalidBump);
        
        let price_data = &mut ctx.accounts.collection_prices;
        let owner = &ctx.accounts.owner;
    
        // Make sure only the original owner can update
        require_keys_eq!(price_data.owner, owner.key(), ErrorCode::Unauthorized);
    
        msg!(
            "Expected prices of length {}, but got {}",
            price_data.size,
            new_prices.len()
        );

        // Check if the size of the new prices array matches the original allocation
        require_eq!(
            new_prices.len(),
            price_data.size as usize,
            ErrorCode::SizeMismatch
        );

        // Validate prices
        for price in &new_prices {
            require_gt!(*price, 0, ErrorCode::InvalidPrice);
            // Check if price is less than 1 SOL (1_000_000_000 lamports)
            if *price >= 1_000_000_000_000_000 {
                return Err(ErrorCode::PriceTooHigh.into());
            }
        }
    
        // Actually update the data
        price_data.prices = new_prices;
    
        // Emit price update event
        emit!(PriceUpdateEvent {
            collection: collection_address,
            owner: owner.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
    
        Ok(())
    }

    pub fn initialize_collection(
        ctx: Context<InitializeCollection>,
        prices: Vec<u64>,
    ) -> Result<()> {
    
        let collection = &mut ctx.accounts.collection_prices;
        collection.bump = ctx.bumps.collection_prices;
        collection.owner = ctx.accounts.owner.key();    
        collection.mint_authority = ctx.accounts.mint_authority.key();
        collection.payment_mint = Pubkey::default(); // Can be set later if needed
        collection.prices = prices.clone();
        collection.size = prices.len() as u16;
        collection.merkle_tree = Pubkey::default(); // Not yet set

        Ok(())
    }

            
    // now needs to be fetched like this:
    // let signer_seeds = &[
    //     b"mint_authority",
    //     collection.key().as_ref(),
    //     &[bump],
    // ];

    pub fn set_merkle_tree(
        ctx: Context<SetMerkleTree>,
        merkle_tree: Pubkey,
    ) -> Result<()> {
        let collection = &mut ctx.accounts.collection_prices;

        let collection_address = ctx.accounts.collection_address.key();

        // Derive expected mint authority PDA
        let (expected_mint_authority, _) = Pubkey::find_program_address(
            &[b"mint_authority", collection_address.as_ref()],
            ctx.program_id,
        );

        // Deserialize TreeConfig account
        let tree_config_data = &mut &**ctx.accounts.tree_config.try_borrow_data()?;
        let tree_config = TreeConfig::deserialize(tree_config_data)
            .map_err(|_| error!(ErrorCode::InvalidTreeConfig))?;

        require_keys_eq!(
            tree_config.tree_delegate,
            expected_mint_authority,
            ErrorCode::InvalidMintAuthority
        );

        // Ensure only the original owner can call this
        require_keys_eq!(collection.owner, ctx.accounts.owner.key(), ErrorCode::Unauthorized);
    
            // Save merkle tree if all checks pass
        let collection_prices = &mut ctx.accounts.collection_prices;
        collection_prices.merkle_tree = merkle_tree;
    
        Ok(())
    }

}

#[account]
pub struct CollectionPrices {
    pub bump: u8,
    pub owner: Pubkey,         // Collection owner
    pub mint_authority: Pubkey,
    pub size: u16,             
    pub payment_mint: Pubkey,
    pub prices: Vec<u64>,
    pub merkle_tree: Pubkey,
}

impl CollectionPrices {
    pub fn dynamic_size(prices_len: usize) -> usize {
        // 4 + prices_len * 8 = Vec<u64> (4 bytes vec length + each u64 is 8 bytes)
        // bump, owner, mint_authority, merkle tree, size, payment mint, all prices size, prices values
        8 + 32 + 32 + 32 + 2 + 32 + 4 + prices_len * 8
    }
}


// Initialize the PDA (Only collection owner can do this)
#[derive(Accounts)]
#[instruction(prices: Vec<u64>)]
pub struct InitializeCollection<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Used only for PDA derivation
    pub collection_address: UncheckedAccount<'info>,

    #[account(init, payer = owner, space = 8 + CollectionPrices::dynamic_size(prices.len()),
        seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    /// CHECK: PDA derived on client and passed in
    #[account(init, payer = owner, space = 8 + 32 + 1,
        seeds = [b"mint_authority", collection_address.key().as_ref()],bump)]
    pub mint_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetMerkleTree<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: This account is only used for modifying its PDA 
    pub collection_address: UncheckedAccount<'info>, 

    #[account(mut,
        seeds = [b"prices", collection_address.key().as_ref()],
        bump = collection_prices.bump,
        has_one = owner,
    )]
    pub collection_prices: Account<'info, CollectionPrices>,

    /// CHECK: Validate manually
    pub tree_config: UncheckedAccount<'info>, // Bubblegum tree config

    pub system_program: Program<'info, System>,
    // do i need to send system program?
}

// Update Prices (Only Owner)
#[derive(Accounts)]
pub struct UpdatePrices<'info> {
    #[account(mut, has_one = owner, seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(mut)]
    pub owner: Signer<'info>,  // Must be the owner

    /// CHECK: This account is only used for deriving the PDA and is not read or written to.
    pub collection_address: AccountInfo<'info>, 

}

// Fetch Prices from PDA
#[derive(Accounts)]
pub struct FetchPrices<'info> {
    #[account(seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    /// CHECK: This account is only used for deriving the PDA and is not read or written to.
    pub collection_address: AccountInfo<'info>, 
}


#[event]
pub struct PriceUpdateEvent {
    pub collection: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}


// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized: You are not the collection owner.")]
    Unauthorized,
    #[msg("Size mismatch: Number of prices does not match expected size.")]
    SizeMismatch,
    #[msg("Stored bump does not match derived bump.")]
    InvalidBump,
    #[msg("Invalid price: Price must be greater than 0.")]
    InvalidPrice,
    #[msg("Price too high: Price must be less than 1 SOL.")]
    PriceTooHigh,
    #[msg("Unable to deserialize tree config. invalid Tree Config.")]
    InvalidTreeConfig,
    #[msg("Mint authority mismatch: Mint authority for Merkle Tree must match collection mint authority.")]
    InvalidMintAuthority
}




