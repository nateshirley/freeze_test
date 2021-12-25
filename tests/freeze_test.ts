import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import * as web3 from "@solana/web3.js";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { FreezeTest } from "../target/types/freeze_test";
import * as spl from "@solana/spl-token";
import {
  Token,
  TOKEN_PROGRAM_ID,
  MintLayout,
  AccountLayout,
} from "@solana/spl-token";
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAccountAddress,
} from "./helpers/tokenHelpers";
import { assert } from "chai";
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
  let mintAuthority: Pda;
  let user = Keypair.generate();
  let TestToken: Token;
  let userTokenAccount: PublicKey;
  let burnerTokenAccount = Keypair.generate();
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
    let [a, b] = await getMembershipAddress(user.publicKey);
    membership = {
      address: a,
      bump: b,
    };
    let [gA, gB] = await getGovernanceMintAuthority();
    mintAuthority = {
      address: gA,
      bump: gB,
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
        9,
        mintAuthority.address,
        mintAuthority.address
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
      ),
      web3.SystemProgram.createAccount({
        fromPubkey: user.publicKey,
        newAccountPubkey: burnerTokenAccount.publicKey,
        space: AccountLayout.span,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(
          AccountLayout.span
        ),
        programId: TOKEN_PROGRAM_ID,
      }),
      Token.createInitAccountInstruction(
        TOKEN_PROGRAM_ID,
        mint.publicKey,
        burnerTokenAccount.publicKey,
        user.publicKey
      )
    );
    await web3.sendAndConfirmTransaction(
      provider.connection,
      configTransaction,
      [user, mint, burnerTokenAccount]
    );

    TestToken = new Token(
      provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      user
    );
  });

  // it("freeze it", async () => {
  //   await TestToken.freezeAccount(userTokenAccount, mintAuthority, []);
  // });

  // it("try transfer", async () => {
  //   await TestToken.transfer(otherTokenAccount, userTokenAccount, user, [], 50);
  //   printTokenBalance(otherTokenAccount, "user token acct");
  // });

  it("initialize", async () => {
    const tx = await program.rpc.initialize(mintAuthority.bump, {
      accounts: {
        initializer: user.publicKey,
        governanceMintAuthority: mintAuthority.address,
        systemProgram: SystemProgram.programId,
      },
      signers: [user],
    });
  });

  it("create membership", async () => {
    // Add your test here.
    const tx = await program.rpc.createMembership(membership.bump, {
      accounts: {
        authority: user.publicKey,
        membership: membership.address,
        governanceTokenAccount: userTokenAccount,
        governanceMint: mint.publicKey,
        governanceMintAuthority: mintAuthority.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      },
      signers: [user],
    });

    let newMemberGovAccount: any = await TestToken.getAccountInfo(
      userTokenAccount
    );
    assert(newMemberGovAccount.amount.toNumber() === 100);
  });

  it("claim membership", async () => {
    await program.rpc.claimMembership({
      accounts: {
        claimant: other.publicKey,
        membership: membership.address,
        governanceMint: mint.publicKey,
        governanceMintAuthority: mintAuthority.address,
        claimantTokenAccount: otherTokenAccount,
        oldMemberTokenAccount: userTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [other],
    });

    let frozenAccount: any = await TestToken.getAccountInfo(userTokenAccount);
    assert(frozenAccount.state === 2, "old member's token account is frozen");

    let claimantTokenInfo = await TestToken.getAccountInfo(otherTokenAccount);
    assert(
      claimantTokenInfo.amount.toNumber() === 100,
      "claimant has 100 gov tokens"
    );
  });

  it("thaw old token account", async () => {
    await program.rpc.thawGovernanceTokenAccount({
      accounts: {
        tokenAccountOwner: user.publicKey,
        tokenAccount: userTokenAccount,
        governanceMint: mint.publicKey,
        governanceMintAuthority: mintAuthority.address,
        burner: burnerTokenAccount.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [user],
    });

    let info2 = await TestToken.getMintInfo();
    console.log("token supply: ", info2.supply.toNumber());
    assert(info2.supply.toNumber() === 100, "token supply 100 after burn");
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
  const getMembershipAddress = (authority: PublicKey) => {
    return PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("member"), authority.toBytes()],
      program.programId
    );
  };
  const getGovernanceMintAuthority = () => {
    return PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("authority")],
      program.programId
    );
  };
});
