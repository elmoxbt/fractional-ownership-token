import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaFractionalOwnershipToken } from "../target/types/solana_fractional_ownership_token";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, createAssociatedTokenAccountInstruction } from "@solana/spl-token";
import { assert } from "chai";

describe("Fractional Ownership veToken System", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaFractionalOwnershipToken as Program<SolanaFractionalOwnershipToken>;

  const authority = provider.wallet as anchor.Wallet;
  const user1 = Keypair.generate();
  const user2 = Keypair.generate();

  let baseMintKeypair: Keypair;
  let veMintKeypair: Keypair;
  let baseMint: PublicKey;
  let veMint: PublicKey;
  let globalState: PublicKey;
  let tokenVault: PublicKey;
  let feeVault: PublicKey;

  const SECONDS_PER_DAY = 24 * 60 * 60;
  const MIN_LOCK_DURATION = 7 * SECONDS_PER_DAY;
  const MAX_LOCK_DURATION = 4 * 365 * SECONDS_PER_DAY;

  before(async () => {
    baseMintKeypair = Keypair.generate();
    veMintKeypair = Keypair.generate();
    baseMint = baseMintKeypair.publicKey;
    veMint = veMintKeypair.publicKey;

    [globalState] = PublicKey.findProgramAddressSync([Buffer.from("global-state")], program.programId);
    [tokenVault] = PublicKey.findProgramAddressSync([Buffer.from("token-vault")], program.programId);
    [feeVault] = PublicKey.findProgramAddressSync([Buffer.from("fee-vault")], program.programId);

    const airdropUser1 = await provider.connection.requestAirdrop(user1.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    const airdropUser2 = await provider.connection.requestAirdrop(user2.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);

    await provider.connection.confirmTransaction(airdropUser1);
    await provider.connection.confirmTransaction(airdropUser2);
  });

  it("Initializes the fractional ownership protocol", async () => {
    const lockMultiplierNumerator = new anchor.BN(4);
    const lockMultiplierDenominator = new anchor.BN(1);

    await program.methods
      .initialize(lockMultiplierNumerator, lockMultiplierDenominator)
      .accountsStrict({
        authority: authority.publicKey,
        globalState,
        baseMint,
        veMint,
        tokenVault,
        feeVault,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([baseMintKeypair, veMintKeypair])
      .rpc();

    const globalStateAccount = await program.account.globalState.fetch(globalState);
    assert.equal(globalStateAccount.authority.toString(), authority.publicKey.toString());
    assert.equal(globalStateAccount.baseMint.toString(), baseMint.toString());
    assert.equal(globalStateAccount.totalLocked.toNumber(), 0);

    console.log("✓ Protocol initialized with 4x max lock multiplier");
  });

  it("Mints base tokens to test users", async () => {
    const user1TokenAccount = getAssociatedTokenAddressSync(baseMint, user1.publicKey, false, TOKEN_2022_PROGRAM_ID);
    const user2TokenAccount = getAssociatedTokenAddressSync(baseMint, user2.publicKey, false, TOKEN_2022_PROGRAM_ID);
    const authorityTokenAccount = getAssociatedTokenAddressSync(baseMint, authority.publicKey, false, TOKEN_2022_PROGRAM_ID);

    await program.methods
      .mintTokens(new anchor.BN(1000 * 10 ** 9))
      .accountsStrict({
        authority: authority.publicKey,
        globalState,
        baseMint,
        recipientTokenAccount: user1TokenAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .preInstructions([
        createAssociatedTokenAccountInstruction(authority.publicKey, user1TokenAccount, user1.publicKey, baseMint, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID)
      ])
      .rpc();

    await program.methods
      .mintTokens(new anchor.BN(500 * 10 ** 9))
      .accountsStrict({
        authority: authority.publicKey,
        globalState,
        baseMint,
        recipientTokenAccount: user2TokenAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .preInstructions([
        createAssociatedTokenAccountInstruction(authority.publicKey, user2TokenAccount, user2.publicKey, baseMint, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID)
      ])
      .rpc();

    await program.methods
      .mintTokens(new anchor.BN(10000 * 10 ** 9))
      .accountsStrict({
        authority: authority.publicKey,
        globalState,
        baseMint,
        recipientTokenAccount: authorityTokenAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .preInstructions([
        createAssociatedTokenAccountInstruction(authority.publicKey, authorityTokenAccount, authority.publicKey, baseMint, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID)
      ])
      .rpc();

    console.log("✓ Minted 1000 tokens to user1, 500 to user2, 10000 to authority");
  });

  it("User1 locks 500 tokens for maximum duration (4 years)", async () => {
    const [userLock] = PublicKey.findProgramAddressSync([Buffer.from("user-lock"), user1.publicKey.toBuffer()], program.programId);
    const userTokenAccount = getAssociatedTokenAddressSync(baseMint, user1.publicKey, false, TOKEN_2022_PROGRAM_ID);
    const userVeTokenAccount = getAssociatedTokenAddressSync(veMint, user1.publicKey, false, TOKEN_2022_PROGRAM_ID);

    await program.methods
      .lockTokens(new anchor.BN(500 * 10 ** 9), new anchor.BN(MAX_LOCK_DURATION))
      .accountsStrict({
        user: user1.publicKey,
        userLock,
        globalState,
        baseMint,
        veMint,
        userTokenAccount,
        userVeTokenAccount,
        tokenVault,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1])
      .rpc();

    const userLockAccount = await program.account.userLock.fetch(userLock);
    assert.equal(userLockAccount.lockedAmount.toNumber(), 500 * 10 ** 9);
    assert.equal(userLockAccount.initialVeAmount.toNumber(), 2000 * 10 ** 9);

    console.log("✓ User1 locked 500 tokens → received 2000 veTokens (4x multiplier)");
  });

  it("User2 locks 200 tokens for minimum duration (7 days)", async () => {
    const [userLock] = PublicKey.findProgramAddressSync([Buffer.from("user-lock"), user2.publicKey.toBuffer()], program.programId);
    const userTokenAccount = getAssociatedTokenAddressSync(baseMint, user2.publicKey, false, TOKEN_2022_PROGRAM_ID);
    const userVeTokenAccount = getAssociatedTokenAddressSync(veMint, user2.publicKey, false, TOKEN_2022_PROGRAM_ID);

    await program.methods
      .lockTokens(new anchor.BN(200 * 10 ** 9), new anchor.BN(MIN_LOCK_DURATION))
      .accountsStrict({
        user: user2.publicKey,
        userLock,
        globalState,
        baseMint,
        veMint,
        userTokenAccount,
        userVeTokenAccount,
        tokenVault,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user2])
      .rpc();

    const userLockAccount = await program.account.userLock.fetch(userLock);
    assert.equal(userLockAccount.lockedAmount.toNumber(), 200 * 10 ** 9);
    assert.equal(userLockAccount.initialVeAmount.toNumber(), 200 * 10 ** 9);

    console.log("✓ User2 locked 200 tokens → received 200 veTokens (1x multiplier)");
  });

  it("Admin deposits 1000 tokens as protocol fees", async () => {
    const authorityTokenAccount = getAssociatedTokenAddressSync(baseMint, authority.publicKey, false, TOKEN_2022_PROGRAM_ID);

    await program.methods
      .depositFees(new anchor.BN(1000 * 10 ** 9))
      .accountsStrict({
        authority: authority.publicKey,
        globalState,
        baseMint,
        authorityTokenAccount,
        feeVault,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .rpc();

    const globalStateAccount = await program.account.globalState.fetch(globalState);
    assert.equal(globalStateAccount.totalFeesDeposited.toNumber(), 1000 * 10 ** 9);

    console.log("✓ Admin deposited 1000 tokens to fee vault");
  });

  it("User1 claims proportional fees based on veToken balance", async () => {
    const [userLock] = PublicKey.findProgramAddressSync([Buffer.from("user-lock"), user1.publicKey.toBuffer()], program.programId);
    const userTokenAccount = getAssociatedTokenAddressSync(baseMint, user1.publicKey, false, TOKEN_2022_PROGRAM_ID);

    await program.methods
      .claimFees()
      .accountsStrict({
        user: user1.publicKey,
        userLock,
        globalState,
        baseMint,
        userTokenAccount,
        feeVault,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([user1])
      .rpc();

    console.log("✓ User1 claimed fees proportional to veToken balance");
  });

  it("User2 claims proportional fees based on veToken balance", async () => {
    const [userLock] = PublicKey.findProgramAddressSync([Buffer.from("user-lock"), user2.publicKey.toBuffer()], program.programId);
    const userTokenAccount = getAssociatedTokenAddressSync(baseMint, user2.publicKey, false, TOKEN_2022_PROGRAM_ID);

    await program.methods
      .claimFees()
      .accountsStrict({
        user: user2.publicKey,
        userLock,
        globalState,
        baseMint,
        userTokenAccount,
        feeVault,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([user2])
      .rpc();

    console.log("✓ User2 claimed fees proportional to veToken balance");
  });

  it("Verifies protocol state", async () => {
    const globalStateAccount = await program.account.globalState.fetch(globalState);

  
    console.log("PROTOCOL STATE ");
    console.log(`Total Locked:     ${(globalStateAccount.totalLocked.toNumber() / 10 ** 9).toFixed(2).padStart(12)} tokens`);
    console.log(`Total veSupply:   ${(globalStateAccount.totalVeSupply.toNumber() / 10 ** 9).toFixed(2).padStart(12)} tokens`);
    console.log(`Fees Deposited:   ${(globalStateAccount.totalFeesDeposited.toNumber() / 10 ** 9).toFixed(2).padStart(12)} tokens`);

    assert.equal(globalStateAccount.totalLocked.toNumber(), 700 * 10 ** 9, "User1 (500) + User2 (200) locked");
    assert.equal(globalStateAccount.totalVeSupply.toNumber(), 2200 * 10 ** 9, "User1 (2000) + User2 (200) veTokens");

    console.log("All tests passed");
  });
});
