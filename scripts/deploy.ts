import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaFractionalOwnershipToken } from "../target/types/solana_fractional_ownership_token";
import { PublicKey, Keypair, Connection, clusterApiUrl } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import * as fs from "fs";

async function main() {
  const connection = new Connection(clusterApiUrl("devnet"), "confirmed");

  const keypairPath = process.env.ANCHOR_WALLET || `${process.env.HOME}/.config/solana/id.json`;
  const keypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(keypairPath, "utf-8")))
  );

  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const programIdlPath = "./target/idl/solana_fractional_ownership_token.json";
  const idl = JSON.parse(fs.readFileSync(programIdlPath, "utf-8"));
  const programId = new PublicKey(idl.address);

  const program = new Program<SolanaFractionalOwnershipToken>(idl, provider);

  console.log("Deploying to Devnet");
  console.log("Program ID:", programId.toString());
  console.log("Deployer:", provider.wallet.publicKey.toString());

  const balance = await connection.getBalance(provider.wallet.publicKey);
  console.log("Deployer balance:", balance / anchor.web3.LAMPORTS_PER_SOL, "SOL");

  if (balance < 0.5 * anchor.web3.LAMPORTS_PER_SOL) {
    console.log("\nRequesting airdrop...");
    const airdropSig = await connection.requestAirdrop(
      provider.wallet.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig);
    console.log("Airdrop confirmed");
  }

  const baseMintKeypair = Keypair.generate();
  const veMintKeypair = Keypair.generate();

  const [globalState] = PublicKey.findProgramAddressSync(
    [Buffer.from("global-state")],
    programId
  );

  const [tokenVault] = PublicKey.findProgramAddressSync(
    [Buffer.from("token-vault")],
    programId
  );

  const [feeVault] = PublicKey.findProgramAddressSync(
    [Buffer.from("fee-vault")],
    programId
  );

  try {
    const existingState: any = await program.account.globalState.fetch(globalState);
    console.log("\n✓ Protocol already initialized!");
    console.log("Base Mint:", existingState.baseMint.toString());
    console.log("VE Mint:", existingState.veMint.toString());
    console.log("Global State:", globalState.toString());
    console.log("Token Vault:", tokenVault.toString());
    console.log("Fee Vault:", feeVault.toString());
    return;
  } catch (err) {
    console.log("\nProtocol not yet initialized, proceeding...");
  }

  console.log("\nInitializing protocol...");
  console.log("Base Mint:", baseMintKeypair.publicKey.toString());
  console.log("VE Mint:", veMintKeypair.publicKey.toString());
  console.log("Global State:", globalState.toString());
  console.log("Token Vault:", tokenVault.toString());
  console.log("Fee Vault:", feeVault.toString());

  const lockMultiplierNumerator = new anchor.BN(4);
  const lockMultiplierDenominator = new anchor.BN(1);

  const tx = await program.methods
    .initialize(lockMultiplierNumerator, lockMultiplierDenominator)
    .accountsStrict({
      authority: provider.wallet.publicKey,
      globalState,
      baseMint: baseMintKeypair.publicKey,
      veMint: veMintKeypair.publicKey,
      tokenVault,
      feeVault,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([baseMintKeypair, veMintKeypair])
    .rpc();

  console.log("\nProtocol initialized successfully!");
  console.log("Transaction signature:", tx);

  const deploymentInfo = {
    network: "devnet",
    programId: programId.toString(),
    authority: provider.wallet.publicKey.toString(),
    globalState: globalState.toString(),
    baseMint: baseMintKeypair.publicKey.toString(),
    veMint: veMintKeypair.publicKey.toString(),
    tokenVault: tokenVault.toString(),
    feeVault: feeVault.toString(),
    lockMultiplier: {
      numerator: lockMultiplierNumerator.toString(),
      denominator: lockMultiplierDenominator.toString(),
    },
    transactionSignature: tx,
    timestamp: new Date().toISOString(),
  };

  const deploymentDir = "./deployments";
  if (!fs.existsSync(deploymentDir)) {
    fs.mkdirSync(deploymentDir);
  }

  const filename = `${deploymentDir}/devnet-${Date.now()}.json`;
  fs.writeFileSync(filename, JSON.stringify(deploymentInfo, null, 2));

  console.log("\nDeployment info saved to:", filename);
  console.log("\nConfiguration:");
  console.log("- Max lock multiplier: 4x");
  console.log("- Min lock duration: 7 days");
  console.log("- Max lock duration: 4 years");
  console.log("\nView transaction:");
  console.log(`https://explorer.solana.com/tx/${tx}?cluster=devnet`);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
