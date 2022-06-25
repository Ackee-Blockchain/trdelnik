use anchor_spl::token;
use escrow;
use fehler::throws;
use program_client::escrow_instruction;
use trdelnik_client::{anyhow::Result, *};

#[throws]
#[fixture]
async fn init_fixture() -> Fixture {
    let mut fixture = Fixture::new();
    // Deploy
    fixture.deploy().await?;
    // Create a PDA authority
    fixture.pda = Pubkey::find_program_address(&[b"escrow"], &escrow::id()).0;
    // Creation of token mint A
    fixture
        .client
        .create_token_mint(&fixture.mint_a, fixture.mint_authority.pubkey(), None, 0)
        .await?;
    // Creation of token mint B
    fixture
        .client
        .create_token_mint(&fixture.mint_b, fixture.mint_authority.pubkey(), None, 0)
        .await?;
    // Creation of alice's and bob's ATAs for token A
    fixture.alice_token_a_account = fixture
        .client
        .create_associated_token_account(&fixture.alice_wallet, fixture.mint_a.pubkey())
        .await?;
    fixture.bob_token_a_account = fixture
        .client
        .create_associated_token_account(&fixture.bob_wallet, fixture.mint_a.pubkey())
        .await?;
    // Creation of alice's and bob's ATAs for token B
    fixture.alice_token_b_account = fixture
        .client
        .create_associated_token_account(&fixture.alice_wallet, fixture.mint_b.pubkey())
        .await?;
    fixture.bob_token_b_account = fixture
        .client
        .create_associated_token_account(&fixture.bob_wallet, fixture.mint_b.pubkey())
        .await?;

    // Mint some tokens
    fixture
        .client
        .mint_tokens(
            fixture.mint_a.pubkey(),
            &fixture.mint_authority,
            fixture.alice_token_a_account,
            500,
        )
        .await?;
    fixture
        .client
        .mint_tokens(
            fixture.mint_b.pubkey(),
            &fixture.mint_authority,
            fixture.bob_token_b_account,
            1000,
        )
        .await?;

    fixture
}

#[trdelnik_test]
async fn test_happy_path1(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // Initialize escrow
    escrow_instruction::initialize_escrow(
        &fixture.client,
        500,
        1000,
        fixture.alice_wallet.pubkey(),
        fixture.alice_token_a_account,
        fixture.alice_token_b_account,
        fixture.escrow_account.pubkey(),
        System::id(),
        token::ID,
        [fixture.alice_wallet.clone(), fixture.escrow_account.clone()],
    )
    .await?;

    let escrow = fixture.get_escrow().await?;
    let alice_token_a_account = fixture
        .get_token_account(fixture.alice_token_a_account)
        .await?;

    assert_eq!(alice_token_a_account.owner, fixture.pda);
    assert_eq!(escrow.initializer_key, fixture.alice_wallet.pubkey());
    assert_eq!(escrow.initializer_amount, 500);
    assert_eq!(escrow.taker_amount, 1000);
    assert_eq!(
        escrow.initializer_deposit_token_account,
        fixture.alice_token_a_account
    );
    assert_eq!(
        escrow.initializer_receive_token_account,
        fixture.alice_token_b_account
    );

    // Exchange
    escrow_instruction::exchange(
        &fixture.client,
        fixture.bob_wallet.pubkey(),
        fixture.bob_token_b_account,
        fixture.bob_token_a_account,
        fixture.alice_token_a_account,
        fixture.alice_token_b_account,
        fixture.alice_wallet.pubkey(),
        fixture.escrow_account.pubkey(),
        fixture.pda,
        token::ID,
        [fixture.bob_wallet.clone()],
    )
    .await?;

    let alice_token_a_account = fixture
        .get_token_account(fixture.alice_token_a_account)
        .await?;
    let alice_token_b_account = fixture
        .get_token_account(fixture.alice_token_b_account)
        .await?;
    let bob_token_a_account = fixture
        .get_token_account(fixture.bob_token_a_account)
        .await?;
    let bob_token_b_account = fixture
        .get_token_account(fixture.bob_token_b_account)
        .await?;

    assert_eq!(alice_token_a_account.owner, fixture.alice_wallet.pubkey());
    assert_eq!(bob_token_a_account.amount, 500);
    assert_eq!(alice_token_a_account.amount, 0);
    assert_eq!(alice_token_b_account.amount, 1000);
    assert_eq!(bob_token_b_account.amount, 0);
}

#[trdelnik_test]
async fn test_happy_path2(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // Initialize escrow
    escrow_instruction::initialize_escrow(
        &fixture.client,
        500,
        1000,
        fixture.alice_wallet.pubkey(),
        fixture.alice_token_a_account,
        fixture.alice_token_b_account,
        fixture.escrow_account.pubkey(),
        System::id(),
        token::ID,
        [fixture.alice_wallet.clone(), fixture.escrow_account.clone()],
    )
    .await?;

    // Cancel
    escrow_instruction::cancel_escrow(
        &fixture.client,
        fixture.alice_wallet.pubkey(),
        fixture.alice_token_a_account,
        fixture.pda,
        fixture.escrow_account.pubkey(),
        token::ID,
        [],
    )
    .await?;

    let alice_token_a_account = fixture
        .get_token_account(fixture.alice_token_a_account)
        .await?;

    assert_eq!(alice_token_a_account.owner, fixture.alice_wallet.pubkey());
    assert_eq!(alice_token_a_account.amount, 500);
}

struct Fixture {
    client: Client,
    program: Keypair,
    // Mint stuff
    mint_a: Keypair,
    mint_b: Keypair,
    mint_authority: Keypair,
    // Escrow
    escrow_account: Keypair,
    // Participants
    alice_wallet: Keypair,
    bob_wallet: Keypair,
    // Token accounts
    alice_token_a_account: Pubkey,
    alice_token_b_account: Pubkey,
    bob_token_a_account: Pubkey,
    bob_token_b_account: Pubkey,
    // PDA authority of escrow
    pda: Pubkey,
}
impl Fixture {
    fn new() -> Self {
        Fixture {
            client: Client::new(system_keypair(0)),
            program: program_keypair(1),

            mint_a: keypair(1),
            mint_b: keypair(2),
            mint_authority: system_keypair(1),

            escrow_account: keypair(99),

            alice_wallet: keypair(21),
            bob_wallet: keypair(22),

            alice_token_a_account: Pubkey::default(),
            alice_token_b_account: Pubkey::default(),
            bob_token_a_account: Pubkey::default(),
            bob_token_b_account: Pubkey::default(),

            pda: Pubkey::default(),
        }
    }

    #[throws]
    async fn deploy(&mut self) {
        self.client
            .airdrop(self.alice_wallet.pubkey(), 5_000_000_000)
            .await?;
        self.client.deploy_by_name(&self.program, "escrow").await?;
    }

    #[throws]
    async fn get_escrow(&self) -> escrow::EscrowAccount {
        self.client
            .account_data::<escrow::EscrowAccount>(self.escrow_account.pubkey())
            .await?
    }

    #[throws]
    async fn get_token_account(&self, key: Pubkey) -> token::TokenAccount {
        self.client.account_data::<token::TokenAccount>(key).await?
    }
}
