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

        price_data.size = new_prices.len() as u16;
        price_data.prices = new_prices;

        Ok(())
    }

    // Initialize the PDA for the first time (only collection owner can do this)
    pub fn initialize_collection_prices(
        ctx: Context<InitializeCollectionPrices>,
        payment_mint: Pubkey, // New parameter for the token mint
        size: u16,
        prices: Vec<u64>,
    ) -> Result<()> {
        let price_data = &mut ctx.accounts.collection_prices;
        let owner = &ctx.accounts.owner;
    
        require_eq!(prices.len(), size as usize, ErrorCode::SizeMismatch);
    
        price_data.owner = owner.key();
        price_data.size = size;
        price_data.payment_mint = payment_mint;
        price_data.prices = prices;
    
        Ok(())
    }
}

#[account]
pub struct CollectionPrices {
    pub owner: Pubkey,         // Collection owner
    pub size: u16,             // Number of items in the collection
    pub payment_mint: Pubkey,  // The SPL token accepted for payment (e.g., USDC)
    pub prices: Vec<u64>,      // Prices in the token's smallest unit (e.g., 6 decimals for USDC)
}

// Initialize the PDA (Only collection owner can do this)
#[derive(Accounts)]
pub struct InitializeCollectionPrices<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 2 + (4 + 100 * 8), 
        seeds = [b"prices", owner.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Fetch Prices from PDA
#[derive(Accounts)]
pub struct FetchPrices<'info> {
    #[account(seeds = [b"prices", collection_prices.owner.as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,
}

// Update Prices (Only Owner)
#[derive(Accounts)]
pub struct UpdatePrices<'info> {
    #[account(mut, has_one = owner, seeds = [b"prices", collection_prices.owner.as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(mut)]
    pub owner: Signer<'info>,
}

// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized: You are not the collection owner.")]
    Unauthorized,
    #[msg("Size mismatch: Number of prices does not match expected size.")]
    SizeMismatch,
}
