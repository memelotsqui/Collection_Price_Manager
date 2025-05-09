import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  createTree,
  fetchTreeConfigFromSeeds,
} from "@metaplex-foundation/mpl-bubblegum";
import {  keypairIdentity, generateSigner } from "@metaplex-foundation/umi";
import type { PublicKey as UmiPublicKey } from '@metaplex-foundation/umi';

import { createCollection } from '@metaplex-foundation/mpl-core'
import { createUmi  } from '@metaplex-foundation/umi-bundle-defaults'

import { PublicKey, SystemProgram, Keypair, LAMPORTS_PER_SOL, Connection, clusterApiUrl } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import fs from "fs";
import { assert, expect } from "chai";
import BN from "bn.js";

import { MPL_BUBBLEGUM_PROGRAM_ID } from "@metaplex-foundation/mpl-bubblegum";

const PROGRAM_ID = new PublicKey("FV2936jpAPgHkguQeefLpMJm6hJdcmHLy2pDCNTb13Xv");

//const BUBBLEGUM_PROGRAM_ID = MPL_BUBBLEGUM_PROGRAM_ID //new PublicKey("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY");
const BUBBLEGUM_PROGRAM_ID = new PublicKey("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY");
const COMPRESSION_PROGRAM = new PublicKey("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK");
const BUBBLEGUM_LOG_WRAPPER_PROGRAM_ID = new PublicKey("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");


function toAnchorPublicKey(umiKey: UmiPublicKey): PublicKey {
  return new PublicKey(umiKey);
}
//const SPL_NOOP_PROGRAM_ID = new PublicKey("cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK");

// https://developers.metaplex.com/core/collections
describe("collection_price_manager", () => {



  //const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  const connection = new anchor.web3.Connection(clusterApiUrl("devnet"), "confirmed");
  const wallet = anchor.Wallet.local();
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  anchor.setProvider(provider);

  const secretKeyString = fs.readFileSync("/mnt/d/Github/anchorWorld/wallets/devent-test-wallets/devenet-wallet-1.json", { encoding: "utf8" });
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  const keypair = Keypair.fromSecretKey(secretKey);

  // Create a Umi instance with the default plugins
  const umi = createUmi('https://api.devnet.solana.com');
  umi.use(keypairIdentity(provider.wallet.payer))
  



  // Collection + payment mints
  const collectionAddress = anchor.web3.Keypair.generate();
  const paymentMint = anchor.web3.Keypair.generate();

  // Merkle tree and config accounts (random Keypairs for init)
  const merkleTree = anchor.web3.Keypair.generate();

  const maxDepth = new BN(14);
  const maxBufferSize = new BN(64);

  console.log("Keypair:", keypair.publicKey.toBase58());

  async function checkBalance(payerPubKey: PublicKey) {
    const balance = await connection.getBalance(payerPubKey);
    console.log("Payer Balance (SOL):", balance / LAMPORTS_PER_SOL);
    console.log(payerPubKey)
  }

  const program = anchor.workspace.CollectionPriceManager as Program;

  let collectionPricesPDA: PublicKey;
  let merkleTreeIndexPDA: PublicKey;
  let mintAuthorityPDA: PublicKey;
  let treeAuthorityPDA: PublicKey;
  let treeConfig: PublicKey;

  //let collectionAddress: PublicKey;
  let usdcMint: PublicKey;
  let ownerTokenAccount: PublicKey;









  let collectionAnchorKey: PublicKey;

  before(async () => {
    await checkBalance(keypair.publicKey);
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


    // CREATE COLLECTION
    const collectionSigner = generateSigner(umi)

    await createCollection(umi, {
      collection: collectionSigner,
      name: 'My Collection',
      uri: 'https://example.com/my-collection.json',
    })

    console.log("collection signer");
    console.log(collectionSigner.publicKey)
    


    collectionAnchorKey = toAnchorPublicKey(collectionSigner.publicKey);

    // Derive PDA for collection prices using collection address
    [collectionPricesPDA] = await PublicKey.findProgramAddressSync(
      [Buffer.from("prices"), collectionAnchorKey.toBuffer()],
      program.programId
    );
    console.log("collectionPricesPDA");
    console.log(collectionPricesPDA.toBase58());

    // mint authority for collection, stored 
    [mintAuthorityPDA] = await PublicKey.findProgramAddressSync(
      [Buffer.from("mint_authority"), collectionAnchorKey.toBuffer()],
      program.programId
    );
    console.log("mint authority");
    console.log(mintAuthorityPDA.toBase58());
  });


  it("Initializes Collection", async () => {
    const prices = [new BN(1000000), new BN(2000000), new BN(1500000)];

    const tx = await program.methods
    .initializeCollection(prices)//collectionAddress.publicKey, usdcMint, size, prices, maxDepth, maxBufferSize)
    .accounts({
      owner: keypair.publicKey,
      collectionAddress: collectionAnchorKey,
      collectionPrices: collectionPricesPDA,
      mintAuthority: mintAuthorityPDA,
      systemProgram: SystemProgram.programId,
    })
    .signers([keypair])
    .rpc();

    console.log("✅ Initialized collection prices:", tx);

    // ✅ Fetch the PDA data immediately after
    const account = await program.account.collectionPrices.fetch(collectionPricesPDA);
    console.log("📦 Fetched PDA account:", account);

    // ✅ Optionally add assertions
    expect(account.owner.toBase58()).to.equal(keypair.publicKey.toBase58());
    expect(account.prices.map(p => p.toNumber())).to.deep.equal([1000000, 2000000, 1500000]);
  }); 

  it ("Sets Merkle Tree", async () => {
    console.log("✅ Set Current Merkle Tree");
  }); 

});