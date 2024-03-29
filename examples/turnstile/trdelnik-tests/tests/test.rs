use fehler::throws;
use program_client::turnstile_instruction;
use trdelnik_client::{anyhow::Result, *};

#[throws]
#[fixture]
async fn init_fixture() -> Fixture {
    // create a test fixture
    let fixture = Fixture {
        client: Client::new(system_keypair(0)),
        program: program_keypair(1),
        state: keypair(42),
        user_initializer: keypair(45),
    };
    fixture
        .client
        .airdrop(fixture.user_initializer.pubkey(), 5_000_000_000)
        .await?;
    // deploy a tested program
    fixture
        .client
        .deploy_by_name(&fixture.program, "turnstile")
        .await?;

    // init instruction call
    turnstile_instruction::initialize(
        &fixture.client,
        fixture.state.pubkey(),
        fixture.user_initializer.pubkey(),
        System::id(),
        [fixture.state.clone(), fixture.user_initializer.clone()],
    )
    .await?;

    fixture
}

#[trdelnik_test]
async fn test_happy_path(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // coin instruction call
    turnstile_instruction::coin(
        &fixture.client,
        "dummy_string".to_owned(),
        fixture.state.pubkey(),
        None,
    )
    .await?;
    // push instruction call
    turnstile_instruction::push(&fixture.client, fixture.state.pubkey(), None).await?;

    // check the test result
    let state = fixture.get_state().await?;

    // after pushing the turnstile should be locked
    assert!(state.locked);
    // the last push was successfull
    assert!(state.res);
}

#[trdelnik_test]
async fn test_unhappy_path(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // pushing without prior coin insertion
    turnstile_instruction::push(&fixture.client, fixture.state.pubkey(), None).await?;

    // check the test result
    let state = fixture.get_state().await?;

    // after pushing the turnstile should be locked
    assert!(state.locked);
    // the last push was successfull
    assert!(!state.res);
}

struct Fixture {
    client: Client,
    program: Keypair,
    state: Keypair,
    user_initializer: Keypair,
}

impl Fixture {
    #[throws]
    async fn get_state(&self) -> turnstile::State {
        self.client.account_data(self.state.pubkey()).await?
    }
}
