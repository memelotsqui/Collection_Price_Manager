use anchor_lang::prelude::*;

declare_id!("FV2936jpAPgHkguQeefLpMJm6hJdcmHLy2pDCNTb13Xv");

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
        let price_data = &mut ctx.accounts.collection_prices;
        let owner = &ctx.accounts.owner;
    
        require_keys_eq!(price_data.owner, owner.key(), ErrorCode::Unauthorized);
        require_eq!(new_prices.len(), price_data.size as usize, ErrorCode::SizeMismatch);
    
        price_data.prices = new_prices;
        Ok(())
    }

    // Initialize the PDA for the first time (only collection owner can do this)
    pub fn initialize_collection_prices(
        ctx: Context<InitializeCollectionPrices>,
        collection_address: Pubkey, 
        payment_mint: Pubkey,
        size: u16,
        prices: Vec<u64>,
    ) -> Result<()> {
        let price_data = &mut ctx.accounts.collection_prices;
        let owner = &ctx.accounts.owner;
        
        require_eq!(prices.len(), size as usize, ErrorCode::SizeMismatch);
        
        price_data.owner = owner.key();
        price_data.collection_address = collection_address;
        price_data.size = size;
        price_data.payment_mint = payment_mint;
        price_data.prices = prices;
        
        Ok(())
    }
}

#[account]
pub struct CollectionPrices {
    pub owner: Pubkey,         // Collection owner
    pub collection_address: Pubkey, // Collection identifier
    pub size: u16,             
    pub payment_mint: Pubkey,
    pub prices: Vec<u64>,
}

// Initialize the PDA (Only collection owner can do this)
#[derive(Accounts)]
pub struct InitializeCollectionPrices<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 32 + 2 + (4 + 100 * 8),
        seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(mut)]
    pub owner: Signer<'info>,  // Must be the creator

    pub system_program: Program<'info, System>,

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

// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized: You are not the collection owner.")]
    Unauthorized,
    #[msg("Size mismatch: Number of prices does not match expected size.")]
    SizeMismatch,
}
