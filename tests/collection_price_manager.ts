import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, Keypair, LAMPORTS_PER_SOL, Connection, clusterApiUrl } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import fs from "fs";
import { assert } from "chai";
import BN from "bn.js";

describe("collection_price_manager", () => {

  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  const wallet = anchor.Wallet.local();
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  anchor.setProvider(provider);

  const secretKeyString = fs.readFileSync("/mnt/d/Github/anchorWorld/wallets/localnet-test-wallets/localnet-2.json", { encoding: "utf8" });
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  const keypair = Keypair.fromSecretKey(secretKey);

  console.log("Keypair:", keypair.publicKey.toBase58());

  async function checkBalance(payerPubKey: PublicKey) {
    const balance = await connection.getBalance(payerPubKey);
    console.log("Payer Balance (SOL):", balance / LAMPORTS_PER_SOL);
  }

  const program = anchor.workspace.CollectionPriceManager as Program;

  let collectionPricesPDA: PublicKey;
  let collectionAddress: PublicKey;
  let usdcMint: PublicKey;
  let ownerTokenAccount: PublicKey;

  before(async () => {
    await checkBalance(keypair.publicKey);
    
    // Simulate a collection address (for testing)
    collectionAddress = anchor.web3.Keypair.generate().publicKey;
    console.log("Collection Address:", collectionAddress.toBase58());

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
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("prices"), collectionAddress.toBuffer()],
      program.programId
    );
    collectionPricesPDA = pda;

    console.log("Collection Prices PDA:", collectionPricesPDA.toBase58());
  });

  it("Initializes collection prices", async () => {
    const size = 3;
    const prices = [new BN(1000000), new BN(2000000), new BN(1500000)];
  
    const tx = await program.methods
      .initializeCollectionPrices(collectionAddress, usdcMint, size, prices)
      .accounts({
        collectionPrices: collectionPricesPDA,
        collectionAddress: collectionAddress,
        owner: keypair.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([keypair])
      .rpc();
  
    console.log("✅ Initialized collection prices:", tx);
  
    const data = await program.account.collectionPrices.fetch(collectionPricesPDA);
    
    assert.strictEqual(data.size, size);
  
    // Convert both to strings for comparison
    assert.deepStrictEqual(data.prices.map(price => price.toString()), prices.map(price => price.toString()));
  
    assert.strictEqual(data.paymentMint.toBase58(), usdcMint.toBase58());
  });
  

  

  it("Fetches collection prices", async () => {
    const data = await program.account.collectionPrices.fetch(collectionPricesPDA);
    console.log("Fetched Prices:", data);
    assert.strictEqual(data.size, 3);
  });

  it("Updates prices", async () => {
    const newPrices = [new BN(5000000), new BN(7000000), new BN(8000000)];
  
    await program.methods
      .updatePrices(newPrices)
      .accounts({
        collectionPrices: collectionPricesPDA,
        collectionAddress: collectionAddress,
        owner: keypair.publicKey,
      })
      .signers([keypair])
      .rpc();
  
    const updatedData = await program.account.collectionPrices.fetch(collectionPricesPDA);
  
    // Convert to string for comparison
    assert.deepStrictEqual(updatedData.prices.map(price => price.toString()), newPrices.map(price => price.toString()));
  });
  
  it("Prevents unauthorized updates", async () => {
    const newPrices = [new BN(1000000), new BN(2000000), new BN(3000000)];
    const fakeUser = anchor.web3.Keypair.generate();
  
    try {
      await program.methods
        .updatePrices(newPrices)
        .accounts({
          collectionPrices: collectionPricesPDA,
          collectionAddress: collectionAddress,
          owner: fakeUser.publicKey,
        })
        .signers([fakeUser])
        .rpc();
  
      assert.fail("Unauthorized update should fail.");
    } catch (err) {
      console.log("✅ Unauthorized update prevented:", err.message);
  
      // Ensure the error contains the expected constraint violation message
      assert.include(err.message, "Constraint has one of constraints was violated");
    }
  });  
});
