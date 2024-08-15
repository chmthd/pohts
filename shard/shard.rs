use crate::poh::generator::PohGenerator;
use crate::block::block::Block;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub amount: u64,
    pub from_shard: usize,
    pub to_shard: usize,
}

#[derive(Debug)]
pub struct Shard {
    pub id: usize,
    generator: PohGenerator,
    epoch: usize,
    transaction_count: usize,
    transaction_pool: Vec<Transaction>, // tx pool vector
    ledger: HashMap<String, u64>,
    pub blocks: Vec<Block>,
    max_transactions_per_block: usize,
    last_block_time: Instant,
    block_time_threshold: Duration,
    epoch_threshold: usize, 
}

impl Shard {
    pub fn new(id: usize, batch_size: usize, max_transactions_per_block: usize) -> Self {
        Shard {
            id,
            generator: PohGenerator::new(batch_size),
            epoch: 0,
            transaction_count: 0,
            transaction_pool: Vec::new(),
            ledger: HashMap::new(),
            blocks: Vec::new(),
            max_transactions_per_block,
            last_block_time: Instant::now(),
            block_time_threshold: Duration::from_secs(10),
            epoch_threshold: 10, /// epoch period depends on tx count for now
        }
    }

    pub fn get_transactions(&self) -> Vec<Transaction> {
        self.transaction_pool.clone()
    }

    pub fn process_transactions(&mut self, transactions: Vec<Transaction>) {
        for tx in transactions {
            if tx.to_shard == self.id {
                self.transaction_pool.push(tx);
                self.transaction_count += 1; 
            } else {
                println!(
                    "Shard {}: Received transaction {} for another shard (Shard {}).",
                    self.id, tx.id, tx.to_shard
                );
            }
        }

        let time_since_last_block = self.last_block_time.elapsed();

        if self.transaction_pool.len() >= self.max_transactions_per_block || time_since_last_block >= self.block_time_threshold {
            self.create_block();
            self.last_block_time = Instant::now();
        }

        // check if the epoch should be incremented after processing transactions
        if self.transaction_count >= self.epoch_threshold {
            self.epoch += 1;
            self.transaction_count = 0;
            println!("Shard {}: Moving to next epoch: {}", self.id, self.epoch);
        }
    }

    fn create_block(&mut self) {
        if self.transaction_pool.is_empty() {
            println!("Shard {}: Error processing transactions: No transactions provided", self.id);
            return;
        }

        let transactions_to_include = self.transaction_pool
            .drain(..std::cmp::min(self.max_transactions_per_block, self.transaction_pool.len()))
            .collect::<Vec<_>>();

        let tx_strings: Vec<String> = transactions_to_include.iter().map(|tx| tx.id.clone()).collect();

        match self.generator.generate_entries(tx_strings) {
            Ok(entries) => {
                let block_number = self.blocks.len() as u64 + 1;
                let previous_hash = if let Some(last_block) = self.blocks.last() {
                    &last_block.block_hash
                } else {
                    "0"
                };
                let block = Block::new(block_number, entries, previous_hash);
                self.blocks.push(block);

                println!("Shard {}: Processed Block {:?}", self.id, self.blocks.last().unwrap());
            }
            Err(e) => {
                println!("Shard {}: Error processing transactions: {}", self.id, e);
            }
        }
    }

    pub fn update_state_from_gossip_data(&mut self, blocks: Vec<Block>) {
        println!("Shard {} is incorporating gossiped blocks.", self.id);
        for block in blocks {
            if !self.blocks.iter().any(|b| b.block_hash == block.block_hash) {
                self.blocks.push(block);
                println!("Shard {}: Added gossiped block to state.", self.id);
            }
        }
    }
}
