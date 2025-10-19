#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/GameMode.quops")]
enum GameMode {
    Normal,
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/Language.quops")]
enum Language {
    English,
    French,
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/RegenChallengeDifficulty.quops")]
enum RegenChallengeDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/ScratchphraseRules.quops")]
struct ScratchphraseRules {
    language: Language,
    game_mode: GameMode,
    regen_challenge_difficulty: RegenChallengeDifficulty,
    regen_challenges: u8,
    solves_per_syllable: i16,
    turn_duration: u16,
    starting_lives: u8,
    max_lives: u8,
    syllable_duration: u8,
    allow_hyphens_and_apostrophes_in_syllables: bool,
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/Role.quops")]
enum Role {
    Player,
    Leader,
    DictionaryEditor,
    Moderator,
    Developer
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/LastRoundWinner.quops")]
struct LastRoundWinner {
    id: i32,
    role: Role,
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/ScratchphraseGameStats.quops")]
struct ScratchphraseGameStats {
    total_time: i32,
    total_words_used: i32,
}

#[derive(Debug, quops::Encode, quops::Decode)]
#[schema(path = "./schemas/ScratchphraseLastRound.quops")]
struct ScratchphraseLastRound {
    winner: Option<LastRoundWinner>,
    game_stats: ScratchphraseGameStats,
}

fn main() {
    let mut value = ScratchphraseRules {
        language: Language::English,
        game_mode: GameMode::Normal,
        regen_challenge_difficulty: RegenChallengeDifficulty::Medium,
        regen_challenges: 2,
        solves_per_syllable: 500,
        turn_duration: 5,
        starting_lives: 2,
        max_lives: 3,
        syllable_duration: 2,
        allow_hyphens_and_apostrophes_in_syllables: false,
    };

    const ITERATIONS: usize = 10_000_000;
    let start = std::time::Instant::now();
    for _ in 0..ITERATIONS {
        std::hint::black_box(quops::encode(std::hint::black_box(&value)).unwrap());
    }
    let end = std::time::Instant::now();
    println!("Encoding {} iterations took: {:?}", ITERATIONS, end - start);
    println!("Average time per encode: {:?}", (end - start) / ITERATIONS as u32);

    let bin = quops::encode(&value).unwrap();
    println!("{:?}, bytes: {}", bin, bin.len());

    value = quops::decode(&bin).unwrap();
    dbg!(&value);
}
