import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import fs from "fs";
import { assert } from "chai";
import BN from "bn.js"; // Ensure BN is imported

describe("collection_price_manager", () => {

  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  const wallet = anchor.Wallet.local(); // Loads local keypair
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  anchor.setProvider(provider);
  
  

  const secretKeyString = fs.readFileSync("/mnt/d/Github/anchorWorld/localnet-test-wallets/localnet-2.json", { encoding: "utf8" });
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  const keypair = Keypair.fromSecretKey(secretKey);

  console.log(secretKeyString);
  console.log(keypair.publicKey);

  async function airdropSol(payerPubKey: PublicKey) {
    console.log("Requesting airdrop...");
    const airdropSignature = await connection.requestAirdrop(payerPubKey, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(airdropSignature, "confirmed");

    const newBalance = await connection.getBalance(payerPubKey);
    console.log("Airdrop successful! New Balance (SOL):", newBalance / LAMPORTS_PER_SOL);
  }

  async function checkBalance(payerPubKey: PublicKey) {
    const balance = await connection.getBalance(payerPubKey);
    console.log("Payer Balance (SOL):", balance / 1_000_000_000);
    if (balance < 0.5){
      console.log("low balance airdropping");
      await airdropSol(payerPubKey);
    }
    console.log(keypair.publicKey)
  }

  

  // const keypairPath = ""

  // const secret = new Uint8Array(JSON.parse(fs.readFileSync(keypairPath)));

  const program = anchor.workspace.CollectionPriceManager as Program;

  let collectionPricesPDA: PublicKey;
  let owner = keypair.publicKey;
  let usdcMint: PublicKey;
  let ownerTokenAccount: PublicKey;

  

  before(async () => {
    await checkBalance(keypair.publicKey);
    // Create a new USDC token for testing
    usdcMint = await createMint(
      provider.connection,
      keypair,
      owner,
      null,
      6 // USDC typically has 6 decimals
    );

    // Create owner's token account to receive payments
    ownerTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      keypair,
      usdcMint,
      owner
    ).then(acc => acc.address);

    // Derive PDA for collection prices
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("prices"), owner.toBuffer()],
      program.programId
    );
    collectionPricesPDA = pda;

    console.log("Collection Prices PDA:", collectionPricesPDA.toBase58());
  });

  it("Initializes collection prices", async () => {
    const size = 3;
    const prices = [new BN(1000000), new BN(2000000), new BN(1500000)]; // USDC amounts in micro-units

    const tx = await program.methods
      .initializeCollectionPrices(usdcMint, size, prices)
      .accounts({
        collectionPrices: collectionPricesPDA,
        owner: owner,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ Initialized collection prices:", tx);

    // Fetch and verify stored data
    const data = await program.account.collectionPrices.fetch(collectionPricesPDA);
    assert.strictEqual(data.size, size);
    assert.deepStrictEqual(data.prices.map(Number), prices);
    assert.strictEqual(data.paymentMint.toBase58(), usdcMint.toBase58());
  });

  it("Fetches collection prices", async () => {
    const data = await program.account.collectionPrices.fetch(collectionPricesPDA);
    console.log("Fetched Prices:", data);
    assert.strictEqual(data.size, 3);
  });

  it("Only owner can modify prices", async () => {
    const newPrices = [new BN(5000000), new BN(7000000), new BN(8000000)];

    await program.methods
      .updatePrices(newPrices)
      .accounts({
        collectionPrices: collectionPricesPDA,
        owner: owner,
      })
      .rpc();

    const updatedData = await program.account.collectionPrices.fetch(collectionPricesPDA);
    assert.deepStrictEqual(updatedData.prices.map(Number), newPrices);
  });

  it("Fails when unauthorized user tries to modify prices", async () => {
    const newPrices = [new BN(1000000), new BN(2000000), new BN(3000000)];
    const fakeUser = anchor.web3.Keypair.generate();

    try {
      await program.methods
        .updatePrices(newPrices)
        .accounts({
          collectionPrices: collectionPricesPDA,
          owner: fakeUser.publicKey, // Unauthorized user
        })
        .signers([fakeUser])
        .rpc();
      assert.fail("Should have thrown an error");
    } catch (err) {
      console.log("✅ Unauthorized update prevented:", err.message);
      assert.include(err.message, "Constraint has one of constraints was violated");
    }
  });
});
