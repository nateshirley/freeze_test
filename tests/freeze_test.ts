import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import * as web3 from "@solana/web3.js";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { FreezeTest } from "../target/types/freeze_test";
import * as spl from "@solana/spl-token";
import { Token, TOKEN_PROGRAM_ID, MintLayout } from "@solana/spl-token";
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAccountAddress,
} from "./helpers/tokenHelpers";
/*

make a token 
put some in an account
freeze the account
try to use the account in an program that requires a balance of > 0

*/

interface Pda {
  address: PublicKey;
  bump: number;
}

describe("freeze_test", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const workspaceParent: any = anchor;
  const program = workspaceParent.workspace.FreezeTest as Program<FreezeTest>;

  let mint = Keypair.generate();
  let mintAuthority = Keypair.generate();
  let user = Keypair.generate();
  let TestToken: Token;
  let userTokenAccount: PublicKey;
  let other = Keypair.generate();
  let otherTokenAccount: PublicKey;
  let membership: Pda;

  it("configure", async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        user.publicKey,
        5 * web3.LAMPORTS_PER_SOL
      ),
      "confirmed"
    );
    userTokenAccount = await getAssociatedTokenAccountAddress(
      user.publicKey,
      mint.publicKey
    );
    otherTokenAccount = await getAssociatedTokenAccountAddress(
      other.publicKey,
      mint.publicKey
    );
    let [a, b] = await getMembershipAddress();
    membership = {
      address: a,
      bump: b,
    };
    let configTransaction = new web3.Transaction().add(
      web3.SystemProgram.createAccount({
        fromPubkey: user.publicKey,
        newAccountPubkey: mint.publicKey,
        space: MintLayout.span,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(
          MintLayout.span
        ),
        programId: TOKEN_PROGRAM_ID,
      }),
      Token.createInitMintInstruction(
        TOKEN_PROGRAM_ID,
        mint.publicKey,
        0,
        mintAuthority.publicKey,
        mintAuthority.publicKey
      ),
      createAssociatedTokenAccountInstruction(
        mint.publicKey,
        userTokenAccount,
        user.publicKey,
        user.publicKey
      ),
      createAssociatedTokenAccountInstruction(
        mint.publicKey,
        otherTokenAccount,
        other.publicKey,
        user.publicKey
      )
    );
    await web3.sendAndConfirmTransaction(
      provider.connection,
      configTransaction,
      [user, mint]
    );

    TestToken = new Token(
      provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      user
    );
    await TestToken.mintTo(userTokenAccount, mintAuthority, [], 100);
    await TestToken.mintTo(otherTokenAccount, mintAuthority, [], 100);

    printTokenBalance(userTokenAccount, "user token acct");
  });

  // it("freeze it", async () => {
  //   await TestToken.freezeAccount(userTokenAccount, mintAuthority, []);
  // });

  // it("try transfer", async () => {
  //   await TestToken.transfer(otherTokenAccount, userTokenAccount, user, [], 50);
  //   printTokenBalance(otherTokenAccount, "user token acct");
  // });

  it("create membership", async () => {
    // Add your test here.
    const tx = await program.rpc.createMembership(membership.bump, {
      accounts: {
        creator: user.publicKey,
        membership: membership.address,
        systemProgram: SystemProgram.programId,
      },
      signers: [user],
    });
  });

  it("claim membership", async () => {
    await program.rpc.claimMembership({
      accounts: {
        claimant: other.publicKey,
        membership: membership.address,
        governanceMint: mint.publicKey,
        claimantTokenAccount: otherTokenAccount,
        oldMemberTokenAccount: userTokenAccount,
      },
      signers: [other],
    });
  });

  const printTokenBalance = async (
    tokenAccount: web3.PublicKey,
    name: string
  ) => {
    let balance = await provider.connection.getTokenAccountBalance(
      tokenAccount
    );
    console.log(name + " balance: " + balance.value.uiAmount);
  };
  const getMembershipAddress = () => {
    return PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("member")],
      program.programId
    );
  };
});
