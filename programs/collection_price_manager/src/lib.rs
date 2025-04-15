use anchor_lang::prelude::*;
use mpl_bubblegum::instructions::CreateTreeConfigCpi;
use mpl_bubblegum::instructions::CreateTreeConfigCpiAccounts;
use mpl_bubblegum::instructions::CreateTreeConfigInstructionArgs ;

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
        let expected_bump = ctx.bumps.collection_prices;
        let stored_bump = ctx.accounts.collection_prices.bump;
    
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
            collection: price_data.collection_address,
            owner: owner.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
    
        Ok(())
    }

    // Initialize the PDA for the first time (only collection owner can do this)
    pub fn initialize_collection_prices(
        ctx: Context<InitializeCollectionPrices>,
        collection_address: Pubkey,
        payment_mint: Pubkey,
        size: u16,
        prices: Vec<u64>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let collection_prices = &mut ctx.accounts.collection_prices;
        collection_prices.bump = ctx.bumps.collection_prices;
        collection_prices.owner = ctx.accounts.owner.key();

        let owner = &ctx.accounts.owner;
        let price_data = &mut ctx.accounts.collection_prices;
        let tree_index = &mut ctx.accounts.merkle_tree_index;
    
        require_eq!(prices.len(), size as usize, ErrorCode::SizeMismatch);
    
        // Save price data
        price_data.owner = owner.key();
        price_data.collection_address = collection_address;
        price_data.size = size;
        price_data.payment_mint = payment_mint;
        price_data.prices = prices;
        price_data.merkle_tree = ctx.accounts.merkle_tree.key();
    
        // Initialize Merkle Tree Index
        tree_index.current_index = 0;
    
        // Call Bubblegum CPI to create Merkle Tree config
        let tree_config = CreateTreeConfigInstructionArgs {
            max_depth,
            max_buffer_size,
            public: Some(false),
        };
    
        let cpi_accounts = CreateTreeConfigCpiAccounts {
            tree_config: &ctx.accounts.tree_config.to_account_info(),
            merkle_tree: &ctx.accounts.merkle_tree.to_account_info(),
            payer: &ctx.accounts.owner.to_account_info(),
            tree_creator: &ctx.accounts.mint_authority.to_account_info(),
            log_wrapper: &ctx.accounts.log_wrapper.to_account_info(),
            compression_program: &ctx.accounts.compression_program.to_account_info(),
            system_program: &ctx.accounts.system_program.to_account_info(),
        };
    
        // Sign with the mint authority PDA
        let binding = ctx.accounts.collection_address.key();
        let signer_seeds = &[
            b"mint_authority",
            binding.as_ref(),
            &[ctx.bumps.mint_authority],
        ];
    
        CreateTreeConfigCpi::new(
            &ctx.accounts.bubblegum_program.to_account_info(),
            cpi_accounts,
            tree_config,
        )
        .invoke_signed(&[signer_seeds])?;
    
        Ok(())
    }

    pub fn rotate_merkle_tree(
        ctx: Context<RotateMerkleTree>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let price_data = &mut ctx.accounts.collection_prices;
        let owner = &ctx.accounts.owner;
    
        // Make sure caller is the collection owner
        require_keys_eq!(price_data.owner, owner.key(), ErrorCode::Unauthorized);
    
        // Save the new Merkle tree address
        price_data.merkle_tree = ctx.accounts.new_merkle_tree.key();
        let tree_index = &mut ctx.accounts.merkle_tree_index;

        // Initialize Merkle Tree Index
        tree_index.current_index = 0;
    
        // Create new tree config
        let tree_config = CreateTreeConfigInstructionArgs {
            max_depth,
            max_buffer_size,
            public: Some(false),
        };
    
        let cpi_accounts = CreateTreeConfigCpiAccounts {
            tree_config: &ctx.accounts.new_tree_config.to_account_info(),
            merkle_tree: &ctx.accounts.new_merkle_tree.to_account_info(),
            payer: &ctx.accounts.owner.to_account_info(),
            tree_creator: &ctx.accounts.mint_authority.to_account_info(),
            log_wrapper: &ctx.accounts.log_wrapper.to_account_info(),
            compression_program: &ctx.accounts.compression_program.to_account_info(),
            system_program: &ctx.accounts.system_program.to_account_info(),
        };
    
        // Sign with the mint authority PDA
        let signer_seeds = &[
            b"mint_authority",
            ctx.accounts.collection_prices.collection_address.as_ref(),
            &[ctx.bumps.mint_authority],
        ];
    
        CreateTreeConfigCpi::new(
            &ctx.accounts.bubblegum_program.to_account_info(),
            cpi_accounts,
            tree_config,
        )
        .invoke_signed(&[signer_seeds])?;
    
        Ok(())
    }
}

#[account]
pub struct CollectionPrices {
    pub bump: u8,
    pub owner: Pubkey,         // Collection owner
    pub collection_address: Pubkey, // Collection identifier
    pub size: u16,             
    pub payment_mint: Pubkey,
    pub prices: Vec<u64>,
    pub merkle_tree: Pubkey,
}

#[account]
pub struct MerkleTreeIndex {
    pub current_index: u64,
}

impl CollectionPrices {
    pub fn dynamic_size(prices_len: usize) -> usize {
        // 4 + prices_len * 8 = Vec<u64> (4 bytes vec length + each u64 is 8 bytes)
        // bump, owner, collection address, merkle tree, size, payment mint, all prices size, prices values
        8 + 32 + 32 + 32 + 2 + 32 + 4 + prices_len * 8
    }
}

impl MerkleTreeIndex {
    pub const MAX_SIZE: usize = 8;
}

#[derive(Accounts)]
#[instruction(max_depth: u32, max_buffer_size: u32)]
pub struct RotateMerkleTree<'info> {
    #[account(mut, has_one = owner)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(init, payer = owner, space = 8 + 8,
        seeds = [b"tree_index", new_merkle_tree.key().as_ref()], bump)]
    pub merkle_tree_index: Account<'info, MerkleTreeIndex>,

    /// CHECK: This is created by the CPI to Bubblegum
    #[account(mut)]
    pub new_merkle_tree: UncheckedAccount<'info>,

    /// CHECK: Tree config will be initialized via CPI
    #[account(mut)]
    pub new_tree_config: UncheckedAccount<'info>,

    /// CHECK: PDA signer for minting, derived from collection address
    #[account(seeds = [b"mint_authority", collection_prices.collection_address.as_ref()], bump)]
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: This is the log wrapper used by Bubblegum
    pub log_wrapper: UncheckedAccount<'info>,

    /// CHECK: This is the Bubblegum program
    pub bubblegum_program: UncheckedAccount<'info>,

    /// CHECK: Compression program used internally by Bubblegum
    pub compression_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// Initialize the PDA (Only collection owner can do this)
#[derive(Accounts)]
#[instruction(size: u16, prices: Vec<u64>)]
pub struct InitializeCollectionPrices<'info> {
    #[account(init, payer = owner, space = 8 + CollectionPrices::dynamic_size(prices.len()),
        seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(init, payer = owner, space = 8 + 8,
        seeds = [b"tree_index", merkle_tree.key().as_ref()], bump)]
    pub merkle_tree_index: Account<'info, MerkleTreeIndex>,

    /// CHECK: PDA signer for minting, derived from collection address
    #[account(seeds = [b"mint_authority", collection_address.key().as_ref()], bump)]
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Will be created during this process
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,

    /// CHECK: Will be derived from Merkle tree and validated inside instruction
    #[account(mut)]
    pub tree_config: UncheckedAccount<'info>,

    /// CHECK: Compression program
    #[account(address = spl_noop::id())]
    pub compression_program: AccountInfo<'info>,

    /// CHECK: Bubblegum program
    #[account(address = mpl_bubblegum::ID)]
    pub bubblegum_program: AccountInfo<'info>,

    /// CHECK: Log wrapper used by Bubblegum
    #[account(address = spl_noop::id())]
    pub log_wrapper: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Used only for PDA derivation
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
    #[msg("Stored bump does not match derived bump.")]
    InvalidBump,
    #[msg("Invalid price: Price must be greater than 0.")]
    InvalidPrice,
    #[msg("Price too high: Price must be less than 1 SOL.")]
    PriceTooHigh,
}

#[event]
pub struct PriceUpdateEvent {
    pub collection: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}
