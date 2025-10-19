use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

#[derive(bitcode::Encode, bitcode::Decode, quops::Encode, quops::Decode, Debug)]
#[schema(path = "/home/adrian/Programming/JavaScript/QUOP/quop/shared/src/quops/GameMode.quops")]
enum GameMode {
    Normal,
}

#[derive(bitcode::Encode, bitcode::Decode, quops::Encode, quops::Decode, Debug)]
#[schema(path = "/home/adrian/Programming/JavaScript/QUOP/quop/shared/src/quops/Language.quops")]
enum Language {
    English,
    French,
}

#[derive(bitcode::Encode, bitcode::Decode, quops::Encode, quops::Decode, Debug)]
#[schema(path = "/home/adrian/Programming/JavaScript/QUOP/quop/shared/src/quops/RegenChallengeDifficulty.quops")]
enum RegenChallengeDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(bitcode::Encode, bitcode::Decode, quops::Encode, quops::Decode, Debug)]
#[schema(path = "/home/adrian/Programming/JavaScript/QUOP/quop/shared/src/quops/ScratchphraseRules.quops")]
struct ScratchphraseRules {
    language: Language,
    game_mode: GameMode,
    regen_challenge_difficulty: RegenChallengeDifficulty,
    regen_challenges: u8,
    solves_per_syllable: i32,
    turn_duration: u16,
    starting_lives: u8,
    max_lives: u8,
    syllable_duration: u8,
    allow_hyphens_and_apostrophes_in_syllables: bool,
}

#[derive(bitcode::Encode, bitcode::Decode, quops::Encode, quops::Decode, Debug)]
#[schema(path = "/home/adrian/Programming/JavaScript/QUOP/quop/shared/src/quops/ChatMessage.quops")]
struct ChatMessage {
    asd: Vec<i32>,
    message: Vec<u8>,
    player_id: u64,
}

fn criterion_benchmark(c: &mut Criterion) {
    let val = ScratchphraseRules {
        language: Language::English,
        game_mode: GameMode::Normal,
        regen_challenge_difficulty: RegenChallengeDifficulty::Easy,
        regen_challenges: 2,
        solves_per_syllable: 500,
        turn_duration: 5,
        starting_lives: 2,
        max_lives: 3,
        syllable_duration: 2,
        allow_hyphens_and_apostrophes_in_syllables: false,
    };

    // let quops_bin = quops::encode(&val).unwrap();
    // let bitcode_bin = bitcode::encode(&val);

    c.bench_function("quops encode", |b| {
        b.iter(|| {
            black_box(quops::encode(black_box(&val)).unwrap());
        });
    });

    c.bench_function("bitcode encode", |b| {
        b.iter(|| {
            black_box(bitcode::encode(black_box(&val)));
        });
    });

    // c.bench_function("quops decode", |b| {
    //     b.iter(|| {
    //         let _ = quops::decode::<ScratchphraseRules>(&quops_bin);
    //     });
    // });
    //
    // c.bench_function("bitcode decode", |b| {
    //     b.iter(|| {
    //         let _ = bitcode::decode::<ScratchphraseRules>(&bitcode_bin);
    //     });
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);