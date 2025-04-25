import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, Keypair, LAMPORTS_PER_SOL, Connection, clusterApiUrl } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import fs from "fs";
import { assert } from "chai";
import BN from "bn.js";

const BUBBLEGUM_PROGRAM_ID = new PublicKey("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY");
const BUBBLEGUM_LOG_WRAPPER_PROGRAM_ID = new PublicKey("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");
const SPL_NOOP_PROGRAM_ID = new PublicKey("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");


describe("collection_price_manager", () => {

  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  //const connection = new anchor.web3.Connection(clusterApiUrl("devnet"), "confirmed");
  const wallet = anchor.Wallet.local();
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  anchor.setProvider(provider);

  const secretKeyString = fs.readFileSync("/mnt/d/Github/anchorWorld/wallets/localnet-test-wallets/localnet-2.json", { encoding: "utf8" });
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  const keypair = Keypair.fromSecretKey(secretKey);

  // Collection + payment mints
  const collectionAddress = anchor.web3.Keypair.generate();
  const paymentMint = anchor.web3.Keypair.generate();

  // Merkle tree and config accounts (random Keypairs for init)
  const merkleTree = anchor.web3.Keypair.generate();
  const treeConfig = anchor.web3.Keypair.generate();

  const maxDepth = new BN(14);
  const maxBufferSize = new BN(64);

  console.log("Keypair:", keypair.publicKey.toBase58());

  async function checkBalance(payerPubKey: PublicKey) {
    const balance = await connection.getBalance(payerPubKey);
    console.log("Payer Balance (SOL):", balance / LAMPORTS_PER_SOL);
  }

  const program = anchor.workspace.CollectionPriceManager as Program;

  let collectionPricesPDA: PublicKey;
  let merkleTreeIndexPDA: PublicKey;
  let mintAuthorityPDA: PublicKey;

  //let collectionAddress: PublicKey;
  let usdcMint: PublicKey;
  let ownerTokenAccount: PublicKey;

  before(async () => {
    await checkBalance(keypair.publicKey);
    
    // Simulate a collection address (for testing)
    //collectionAddress = anchor.web3.Keypair.generate().publicKey;
    //console.log("Collection Address:", collectionAddress.toBase58());

    // Create a new USDC token for testing
    usdcMint = await createMint(
      provider.connection,
      keypair,
      keypair.publicKey,
      null,
      6
    );

    // Create token account for payments
    ownerTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      keypair,
      usdcMint,
      keypair.publicKey
    ).then(acc => acc.address);

    // Derive PDA for collection prices using collection address
    [collectionPricesPDA] = await PublicKey.findProgramAddressSync(
      [Buffer.from("prices"), collectionAddress.publicKey.toBuffer()],
      program.programId
    );
    [mintAuthorityPDA] = await PublicKey.findProgramAddressSync(
      [Buffer.from("mint_authority"), collectionAddress.publicKey.toBuffer()],
      program.programId
    );
    [merkleTreeIndexPDA] = await PublicKey.findProgramAddressSync(
      [Buffer.from("tree_index"), merkleTree.publicKey.toBuffer()],
      program.programId
    );
    //collectionPricesPDA = pda;

    console.log("Collection Prices PDA:", collectionPricesPDA.toBase58());
  });

  it("Fetches prices", async () => {
    const tx = await program.methods
      .fetchPrices()
      .accounts({
        collectionPrices: collectionPricesPDA,
        collectionAddress: collectionAddress.publicKey,
      })
  })

  it("Initializes collection prices", async () => {
    const size = 3;
    const prices = [new BN(1000000), new BN(2000000), new BN(1500000)];
  
    const tx = await program.methods
      .initializeCollectionPrices(size, prices, maxDepth, maxBufferSize)//collectionAddress.publicKey, usdcMint, size, prices, maxDepth, maxBufferSize)
      .accounts({
        // collection_prices
        collectionPrices: collectionPricesPDA,
        // merkle_tree_index
        merkleTreeIndex: merkleTreeIndexPDA,
        // mint_authority
        mintAuthority: mintAuthorityPDA,
        // owner
        owner: keypair.publicKey,
        // merkle_tree
        merkleTree: merkleTree.publicKey,
        // tree_config
        treeConfig: treeConfig.publicKey,
        // compression_program
        compressionProgram: SPL_NOOP_PROGRAM_ID,
        // bubblegum_program
        bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
        // log_wrapper
        logWrapper: BUBBLEGUM_LOG_WRAPPER_PROGRAM_ID ,
        // system_program
        systemProgram: SystemProgram.programId,
        // collection_address
        collectionAddress: collectionAddress.publicKey,
        
      })
      .signers([keypair])
      .rpc();
  
    console.log("✅ Initialized collection prices:", tx);
  
    //const data = await program.account.collectionPrices.fetch(collectionPricesPDA);
    
    //assert.strictEqual(data.size, size);
  
    // Convert both to strings for comparison
    //assert.deepStrictEqual(data.prices.map(price => price.toString()), prices.map(price => price.toString()));
  
    //assert.strictEqual(data.paymentMint.toBase58(), usdcMint.toBase58());
  });
  

  


});