use prettytable::{row, Table};
use std::collections::HashMap;

/// Represents fuzzing statistics, specifically tracking the number of times
/// an instruction was invoked and successfully executed.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IterationStats {
    pub invoked: u64,
    pub successfully_invoked: u64,
}

/// Manages and aggregates statistics for fuzzing instructions.
#[derive(Debug, Default)]
pub struct FuzzingStatistics {
    pub instructions: HashMap<String, IterationStats>,
}

impl FuzzingStatistics {
    /// Constructs a new, empty `FuzzingStatistics`.
    pub fn new() -> Self {
        let empty_instructions = HashMap::<String, IterationStats>::default();
        Self {
            instructions: empty_instructions,
        }
    }
    /// Outputs the statistics as a serialized JSON string.
    pub fn output_serialized(&self) {
        let serialized = serde_json::to_string(&self.instructions).unwrap();
        println!("{}", serialized);
    }

    /// Increments the invocation count for a given instruction.
    /// # Arguments
    /// * `instruction` - The instruction to increment the count for.
    pub fn increase_invoked(&mut self, instruction: String) {
        self.instructions
            .entry(instruction)
            .and_modify(|iterations_stats| iterations_stats.invoked += 1)
            .or_insert(IterationStats {
                invoked: 1,
                successfully_invoked: 0,
            });
    }
    /// Increments the successful invocation count for a given instruction.
    /// # Arguments
    /// * `instruction` - The instruction to increment the successful count for.
    pub fn increase_successfully_invoked(&mut self, instruction: String) {
        self.instructions
            .entry(instruction)
            .and_modify(|iterations_stats| iterations_stats.successfully_invoked += 1)
            .or_insert(
                // this should not occure as instruction has to be invoked
                // and then successfully_invoked
                IterationStats {
                    invoked: 1,
                    successfully_invoked: 1,
                },
            );
    }

    /// Inserts or updates instructions with statistics provided in a serialized string.
    /// # Arguments
    /// * `serialized_iteration` - The serialized statistics to insert or update.
    pub fn insert_serialized(&mut self, serialized_iteration: &str) {
        let result = serde_json::from_str::<HashMap<String, IterationStats>>(serialized_iteration);

        if let Ok(deserialized_instruction) = result {
            for (key, value) in deserialized_instruction {
                self.instructions
                    .entry(key)
                    .and_modify(|instruction_stats| {
                        instruction_stats.invoked += value.invoked;
                        instruction_stats.successfully_invoked += value.successfully_invoked;
                    })
                    .or_insert_with(|| IterationStats {
                        invoked: value.invoked,
                        successfully_invoked: value.successfully_invoked,
                    });
            }
        }
    }
    /// Displays the collected statistics in a formatted table.
    pub fn show_table(&self) {
        let mut table = Table::new();
        table.add_row(row!["Instruction", "Invoked", "Successfully Invoked"]);
        for (instruction, stats) in &self.instructions {
            table.add_row(row![instruction, stats.invoked, stats.successfully_invoked]);
        }
        table.printstd();
    }
}
