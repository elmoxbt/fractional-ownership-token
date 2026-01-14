import { FC, useState, useEffect } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { PublicKey } from '@solana/web3.js';
import * as anchor from '@coral-xyz/anchor';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, createAssociatedTokenAccountInstruction } from '@solana/spl-token';
import idl from '../idl.json';

const PROGRAM_ID = new PublicKey(idl.address);
const IDL = idl as Idl;

interface ProtocolState {
  totalLocked: number;
  totalVeSupply: number;
  totalFeesDeposited: number;
  cumulativeFeePerVeToken: any;
}

interface UserState {
  lockedAmount: number;
  veAmount: number;
  unlockTime: Date;
  feeDebt: any;
}

interface SuccessMessage {
  message: string;
  txSignature: string;
}

const VeTokenApp: FC = () => {
  const { connection } = useConnection();
  const wallet = useWallet();
  const [program, setProgram] = useState<Program | null>(null);
  const [protocolState, setProtocolState] = useState<ProtocolState | null>(null);
  const [userState, setUserState] = useState<UserState | null>(null);
  const [lockAmount, setLockAmount] = useState('');
  const [lockDuration, setLockDuration] = useState('7');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState<SuccessMessage | null>(null);
  const [feeVaultBalance, setFeeVaultBalance] = useState(0);

  useEffect(() => {
    if (wallet.publicKey && wallet.signTransaction && wallet.signAllTransactions) {
      const provider = new AnchorProvider(
        connection,
        wallet as any,
        { commitment: 'confirmed' }
      );
      const programInstance = new Program(IDL, provider);
      setProgram(programInstance);
    }
  }, [connection, wallet]);

  useEffect(() => {
    if (program && wallet.publicKey) {
      fetchProtocolState();
      fetchUserState();
    }
  }, [program, wallet.publicKey]);

  const fetchProtocolState = async () => {
    if (!program) return;

    try {
      const [globalState] = PublicKey.findProgramAddressSync(
        [Buffer.from('global-state')],
        PROGRAM_ID
      );

      const state: any = await (program.account as any).globalState.fetch(globalState);

      const [feeVault] = PublicKey.findProgramAddressSync(
        [Buffer.from('fee-vault')],
        PROGRAM_ID
      );

      const feeVaultAccount = await connection.getAccountInfo(feeVault);
      let vaultBalance = 0;
      if (feeVaultAccount && feeVaultAccount.data.length > 0) {
        const accountData = Buffer.from(feeVaultAccount.data);
        const amount = accountData.readBigUInt64LE(64);
        vaultBalance = Number(amount) / 1e9;
      }

      setFeeVaultBalance(vaultBalance);
      setProtocolState({
        totalLocked: state.totalLocked.toNumber() / 1e9,
        totalVeSupply: state.totalVeSupply.toNumber() / 1e9,
        totalFeesDeposited: state.totalFeesDeposited.toNumber() / 1e9,
        cumulativeFeePerVeToken: state.cumulativeFeePerVeToken,
      });
    } catch (err) {
      console.error('Failed to fetch protocol state:', err);
    }
  };

  const fetchUserState = async () => {
    if (!program || !wallet.publicKey) return;

    try {
      const [userLock] = PublicKey.findProgramAddressSync(
        [Buffer.from('user-lock'), wallet.publicKey.toBuffer()],
        PROGRAM_ID
      );

      const state: any = await (program.account as any).userLock.fetch(userLock);
      setUserState({
        lockedAmount: state.lockedAmount.toNumber() / 1e9,
        veAmount: state.initialVeAmount.toNumber() / 1e9,
        unlockTime: new Date(state.unlockTime.toNumber() * 1000),
        feeDebt: state.feeDebt,
      });
    } catch (err) {
      setUserState(null);
    }
  };

  const handleLockTokens = async () => {
    if (!program || !wallet.publicKey) return;

    setLoading(true);
    setError('');
    setSuccess(null);

    try {
      const [globalState] = PublicKey.findProgramAddressSync(
        [Buffer.from('global-state')],
        PROGRAM_ID
      );

      const state: any = await (program.account as any).globalState.fetch(globalState);
      const baseMint = state.baseMint;
      const veMint = state.veMint;

      const [userLock] = PublicKey.findProgramAddressSync(
        [Buffer.from('user-lock'), wallet.publicKey.toBuffer()],
        PROGRAM_ID
      );

      const [tokenVault] = PublicKey.findProgramAddressSync(
        [Buffer.from('token-vault')],
        PROGRAM_ID
      );

      const userTokenAccount = getAssociatedTokenAddressSync(
        baseMint,
        wallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const userVeTokenAccount = getAssociatedTokenAddressSync(
        veMint,
        wallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const amount = new anchor.BN(parseFloat(lockAmount) * 1e9);
      const duration = new anchor.BN(parseInt(lockDuration) * 24 * 60 * 60);

      const preInstructions = [];

      const userTokenAccountInfo = await connection.getAccountInfo(userTokenAccount);
      if (!userTokenAccountInfo) {
        preInstructions.push(
          createAssociatedTokenAccountInstruction(
            wallet.publicKey,
            userTokenAccount,
            wallet.publicKey,
            baseMint,
            TOKEN_2022_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
          )
        );
      }

      const tx = await program.methods
        .lockTokens(amount, duration)
        .accountsStrict({
          user: wallet.publicKey,
          userLock,
          globalState,
          baseMint,
          veMint,
          userTokenAccount,
          userVeTokenAccount,
          tokenVault,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .preInstructions(preInstructions)
        .rpc({ skipPreflight: false, commitment: 'confirmed' });

      await connection.confirmTransaction(tx, 'confirmed');

      await fetchProtocolState();
      await fetchUserState();
      setLockAmount('');
      setSuccess({
        message: 'Successfully locked tokens',
        txSignature: tx,
      });
    } catch (err: any) {
      console.error('Lock tokens error:', err);
      setError(err.message || 'Transaction failed');
    } finally {
      setLoading(false);
    }
  };

  const handleMintTokens = async () => {
    if (!program || !wallet.publicKey) return;

    setLoading(true);
    setError('');
    setSuccess(null);

    try {
      const [globalState] = PublicKey.findProgramAddressSync(
        [Buffer.from('global-state')],
        PROGRAM_ID
      );

      const state: any = await (program.account as any).globalState.fetch(globalState);
      const baseMint = state.baseMint;

      const userTokenAccount = getAssociatedTokenAddressSync(
        baseMint,
        wallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const preInstructions = [];
      const userTokenAccountInfo = await connection.getAccountInfo(userTokenAccount);
      if (!userTokenAccountInfo) {
        preInstructions.push(
          createAssociatedTokenAccountInstruction(
            wallet.publicKey,
            userTokenAccount,
            wallet.publicKey,
            baseMint,
            TOKEN_2022_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
          )
        );
      }

      const amount = new anchor.BN(1000 * 1e9);

      const tx = await program.methods
        .mintTokens(amount)
        .accountsStrict({
          authority: wallet.publicKey,
          globalState,
          baseMint,
          recipientTokenAccount: userTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .preInstructions(preInstructions)
        .rpc({ skipPreflight: false, commitment: 'confirmed' });

      await connection.confirmTransaction(tx, 'confirmed');

      setSuccess({
        message: 'Successfully minted 1000 tokens',
        txSignature: tx,
      });
    } catch (err: any) {
      console.error('Mint tokens error:', err);
      setError(err.message || 'Transaction failed');
    } finally {
      setLoading(false);
    }
  };

  const handleDepositFees = async () => {
    if (!program || !wallet.publicKey) return;

    setLoading(true);
    setError('');
    setSuccess(null);

    try {
      const [globalState] = PublicKey.findProgramAddressSync(
        [Buffer.from('global-state')],
        PROGRAM_ID
      );

      const state: any = await (program.account as any).globalState.fetch(globalState);
      const baseMint = state.baseMint;

      const [feeVault] = PublicKey.findProgramAddressSync(
        [Buffer.from('fee-vault')],
        PROGRAM_ID
      );

      const authorityTokenAccount = getAssociatedTokenAddressSync(
        baseMint,
        wallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const amount = new anchor.BN(100 * 1e9);

      const tx = await program.methods
        .depositFees(amount)
        .accountsStrict({
          authority: wallet.publicKey,
          globalState,
          baseMint,
          authorityTokenAccount,
          feeVault,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .rpc({ skipPreflight: false, commitment: 'confirmed' });

      await connection.confirmTransaction(tx, 'confirmed');

      await fetchProtocolState();
      setSuccess({
        message: 'Successfully deposited 100 tokens as fees',
        txSignature: tx,
      });
    } catch (err: any) {
      console.error('Deposit fees error:', err);
      let errorMessage = 'Transaction failed';

      if (err.message?.includes('constraint')) {
        errorMessage = 'Only the protocol authority can deposit fees';
      } else if (err.message) {
        errorMessage = err.message;
      }

      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  };

  const handleClaimFees = async () => {
    if (!program || !wallet.publicKey) return;

    setLoading(true);
    setError('');
    setSuccess(null);

    try {
      const [globalState] = PublicKey.findProgramAddressSync(
        [Buffer.from('global-state')],
        PROGRAM_ID
      );

      const state: any = await (program.account as any).globalState.fetch(globalState);
      const baseMint = state.baseMint;

      const [userLock] = PublicKey.findProgramAddressSync(
        [Buffer.from('user-lock'), wallet.publicKey.toBuffer()],
        PROGRAM_ID
      );

      const [feeVault] = PublicKey.findProgramAddressSync(
        [Buffer.from('fee-vault')],
        PROGRAM_ID
      );

      const userTokenAccount = getAssociatedTokenAddressSync(
        baseMint,
        wallet.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID
      );

      const tx = await program.methods
        .claimFees()
        .accountsStrict({
          user: wallet.publicKey,
          userLock,
          globalState,
          baseMint,
          userTokenAccount,
          feeVault,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .rpc({ skipPreflight: false, commitment: 'confirmed' });

      await connection.confirmTransaction(tx, 'confirmed');

      await fetchProtocolState();
      await fetchUserState();
      setSuccess({
        message: 'Successfully claimed fees',
        txSignature: tx,
      });
    } catch (err: any) {
      console.error('Claim fees error:', err);
      let errorMessage = 'Transaction failed';

      if (err.message?.includes('NoVotingPower')) {
        errorMessage = 'You have no voting power. Lock tokens first.';
      } else if (err.message?.includes('NoFeesToClaim')) {
        errorMessage = 'No fees available to claim.';
      } else if (err.message?.includes('NoExistingLock')) {
        errorMessage = 'You need to lock tokens first.';
      } else if (err.message) {
        errorMessage = err.message;
      }

      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="container">
      <div className="header">
        <h1>veToken Fractional Ownership</h1>
        <p>Lock tokens to earn voting power and claim protocol fees</p>
      </div>

      <div className="wallet-section">
        <div>
          {wallet.publicKey && (
            <div>
              <p>Connected: {wallet.publicKey.toString().slice(0, 4)}...{wallet.publicKey.toString().slice(-4)}</p>
            </div>
          )}
        </div>
        <WalletMultiButton />
      </div>

      {wallet.publicKey && protocolState && (
        <>
          <div className="stats-grid">
            <div className="stat-card">
              <h3>Total Locked</h3>
              <p>{protocolState.totalLocked.toFixed(2)}</p>
            </div>
            <div className="stat-card">
              <h3>Total veTokens</h3>
              <p>{protocolState.totalVeSupply.toFixed(2)}</p>
            </div>
            <div className="stat-card">
              <h3>Available Fees</h3>
              <p>{feeVaultBalance.toFixed(2)}</p>
            </div>
          </div>

          {userState && userState.lockedAmount > 0 && (
            <div className="card">
              <h2>Your Position</h2>
              <div className="stats-grid">
                <div className="stat-card">
                  <h3>Locked Amount</h3>
                  <p>{userState.lockedAmount.toFixed(2)}</p>
                </div>
                <div className="stat-card">
                  <h3>Your veTokens</h3>
                  <p>{userState.veAmount.toFixed(2)}</p>
                </div>
                <div className="stat-card">
                  <h3>Your Share</h3>
                  <p>{protocolState ? ((userState.veAmount / protocolState.totalVeSupply) * 100).toFixed(2) : 0}%</p>
                </div>
                <div className="stat-card">
                  <h3>Claimable Fees</h3>
                  <p>{(() => {
                    if (!protocolState || !userState.feeDebt) return '0.00';
                    const feePerVeToken = protocolState.cumulativeFeePerVeToken - userState.feeDebt;
                    const claimable = (userState.veAmount * Number(feePerVeToken)) / 1e18;
                    return claimable.toFixed(2);
                  })()}</p>
                </div>
                <div className="stat-card">
                  <h3>Unlock Time</h3>
                  <p>{userState.unlockTime.toLocaleDateString()}</p>
                </div>
              </div>
              <button className="btn btn-primary" onClick={handleClaimFees} disabled={loading}>
                {loading ? 'Processing...' : 'Claim Fees'}
              </button>
            </div>
          )}

          <div className="card">
            <h2>Get Tokens</h2>
            <button className="btn btn-primary" onClick={handleMintTokens} disabled={loading}>
              {loading ? 'Processing...' : 'Mint 1000 Tokens'}
            </button>
            <div className="info">
              Mint test tokens to your wallet before locking
            </div>
          </div>

          <div className="card">
            <h2>Deposit Protocol Fees</h2>
            <button className="btn btn-primary" onClick={handleDepositFees} disabled={loading}>
              {loading ? 'Processing...' : 'Deposit 100 Tokens as Fees'}
            </button>
            <div className="info">
              Only the protocol authority can deposit fees.
            </div>
          </div>

          <div className="card">
            <h2>Lock Tokens</h2>
            <div className="form-group">
              <label>Amount</label>
              <input
                type="number"
                placeholder="Enter amount to lock"
                value={lockAmount}
                onChange={(e) => setLockAmount(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>Lock Duration (days)</label>
              <select value={lockDuration} onChange={(e) => setLockDuration(e.target.value)}>
                <option value="7">7 days (1x multiplier)</option>
                <option value="30">30 days</option>
                <option value="90">90 days</option>
                <option value="180">180 days</option>
                <option value="365">1 year (2x multiplier)</option>
                <option value="730">2 years (3x multiplier)</option>
                <option value="1460">4 years (4x multiplier)</option>
              </select>
            </div>
            <button
              className="btn btn-primary"
              onClick={handleLockTokens}
              disabled={loading || !lockAmount}
            >
              {loading ? 'Processing...' : 'Lock Tokens'}
            </button>
            <div className="info">
              Longer lock periods earn higher voting power multipliers (up to 4x for 4 years)
            </div>
          </div>

          {success && (
            <div className="success">
              {success.message}
              <br />
              <a
                href={`https://explorer.solana.com/tx/${success.txSignature}?cluster=devnet`}
                target="_blank"
                rel="noopener noreferrer"
              >
                View transaction
              </a>
            </div>
          )}

          {error && <div className="error">{error}</div>}
        </>
      )}

      {!wallet.publicKey && (
        <div className="loading">
          Connect your wallet to get started
        </div>
      )}
    </div>
  );
};

export default VeTokenApp;
