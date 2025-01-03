use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub number: u64,
    pub balance: u64,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let account = next_account_info(accounts_iter)?;

    // Ensure the account is writable
    if !account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure the account is owned by the program
    if *account.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize the instruction data
    let instruction: CommandInstruction =
        deserialize(instruction_data).map_err(|_| ProgramError::InvalidAccountData)?;

    // Validate the instruction parameters
    match instruction.command {
        Command::Deposit { amount } => {
            if amount == 0 {
                return Err(ProgramError::InvalidInstructionData); // Invalid amount for deposit
            }
        }
        Command::Withdraw { amount } => {
            if amount == 0 {
                return Err(ProgramError::InvalidInstructionData); // Invalid amount for withdrawal
            }
        }
        Command::CheckBalance => {
            // No parameters to validate for CheckBalance
        }
    }

    // Ensure the account is owned by the instruction program
    if *account.owner != instruction.program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Read existing data or initialize if empty
    let mut data = if account.data.borrow().len() == 0 {
        Data {
            number: 0,
            balance: 0,
        }
    } else {
        deserialize(&account.data.borrow()).map_err(|_| ProgramError::InvalidAccountData)?
    };

    match instruction.command {
        Command::Deposit { amount } => {
            data.balance += amount;
            msg!("Deposited {} SOL. New balance: {}", amount, data.balance);
        }
        Command::Withdraw { amount } => {
            if amount > data.balance {
                return Err(ProgramError::InsufficientFunds);
            }
            data.balance -= amount;
            msg!("Withdrew {} SOL. New balance: {}", amount, data.balance);
        }
        Command::CheckBalance => {
            msg!("Current balance: {}", data.balance);
        }
    }

    // Serialize and save the data back to the account
    let serialized_data = serialize(&data).map_err(|_| ProgramError::InvalidAccountData)?;
    account.data.borrow_mut().copy_from_slice(&serialized_data);

    Ok(())
}

// Define the instruction data structure
#[derive(Deserialize, Serialize, Debug)]
pub enum Command {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
    CheckBalance,
}

// Define the instruction struct
#[derive(Deserialize, Serialize, Debug)]
pub struct CommandInstruction {
    pub command: Command,
    program_id: Pubkey,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::serialize;
    use solana_program_test::*;
    use solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_deposit() {
        let program_id = Pubkey::new_unique();
        let mut program_test =
            ProgramTest::new("program_name", program_id, processor!(process_instruction));

        // Create an account to hold the data
        let user_account = Keypair::new();
        let initial_balance = 1_000_000_000; // Enough lamports to be rent-exempt
        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: initial_balance,
                data: vec![0; 16], // Allocate space for serialized Data struct (u64 + u64)
                owner: program_id,
                ..Account::default()
            },
        );

        let (banks_client, payer, recent_blockhash) = program_test.start().await;

        // Deposit 100 SOL
        let instruction = CommandInstruction {
            program_id,
            command: Command::Deposit { amount: 100 },
        };
        let instruction_data = serialize(&instruction).unwrap();

        // Create the transaction to call the program
        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![AccountMeta::new(user_account.pubkey(), false)],
                data: instruction_data,
            }],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        // Process the transaction with error handling
        match banks_client.process_transaction(transaction).await {
            Ok(_) => {
                // Transaction succeeded, check the balance
                let account_data = banks_client
                    .get_account(user_account.pubkey())
                    .await
                    .unwrap()
                    .unwrap();
                let data: Data = deserialize(&account_data.data).unwrap();
                assert_eq!(data.balance, 100);
            }
            Err(e) => {
                // Handle the error appropriately
                eprintln!("Transaction failed: {:?}", e);
                panic!("Transaction failed: {:?}", e); // You can choose to panic or handle it differently
            }
        }
    }

    #[tokio::test]
    async fn test_withdraw() {
        let program_id = Pubkey::new_unique();
        let mut program_test =
            ProgramTest::new("program_name", program_id, processor!(process_instruction));

        // Create an account to hold the data
        let user_account = Keypair::new();
        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: 1_000_000_000,
                data: serialize(&Data {
                    number: 0,
                    balance: 100,
                })
                .unwrap(),
                owner: program_id,
                ..Account::default()
            },
        );

        let (banks_client, payer, recent_blockhash) = program_test.start().await;

        // Withdraw 50 SOL
        let instruction = CommandInstruction {
            program_id,
            command: Command::Withdraw { amount: 50 },
        };
        let instruction_data = serialize(&instruction).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![AccountMeta::new(user_account.pubkey(), false)],
                data: instruction_data,
            }],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        banks_client.process_transaction(transaction).await.unwrap();

        // Check the balance
        let account_data = banks_client
            .get_account(user_account.pubkey())
            .await
            .unwrap()
            .unwrap();
        let data: Data = deserialize(&account_data.data).unwrap();
        assert_eq!(data.balance, 50);
    }

    #[tokio::test]
    async fn test_check_balance() {
        let program_id = Pubkey::new_unique();
        let mut program_test =
            ProgramTest::new("program_name", program_id, processor!(process_instruction));

        // Create an account to hold the data
        let user_account = Keypair::new();
        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: 1_000_000_000,
                data: serialize(&Data {
                    number: 0,
                    balance: 150,
                })
                .unwrap(),
                owner: program_id,
                ..Account::default()
            },
        );

        let (banks_client, payer, recent_blockhash) = program_test.start().await;

        // Check balance
        let instruction = CommandInstruction {
            program_id,
            command: Command::CheckBalance,
        };
        let instruction_data = serialize(&instruction).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![AccountMeta::new(user_account.pubkey(), false)],
                data: instruction_data,
            }],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        banks_client.process_transaction(transaction).await.unwrap();

        // The balance should still be 150
        let account_data = banks_client
            .get_account(user_account.pubkey())
            .await
            .unwrap()
            .unwrap();
        let data: Data = deserialize(&account_data.data).unwrap();
        assert_eq!(data.balance, 150);
    }

    #[tokio::test]
    async fn test_insufficient_funds() {
        let program_id = Pubkey::new_unique();
        let mut program_test =
            ProgramTest::new("program_name", program_id, processor!(process_instruction));

        // Create an account with a balance of 50
        let user_account = Keypair::new();
        program_test.add_account(
            user_account.pubkey(),
            Account {
                lamports: 1_000_000_000,
                data: serialize(&Data {
                    number: 0,
                    balance: 50,
                })
                .unwrap(),
                owner: program_id,
                ..Account::default()
            },
        );

        let (banks_client, payer, recent_blockhash) = program_test.start().await;

        // Attempt to withdraw 100 SOL (should fail)
        let instruction = CommandInstruction {
            program_id,
            command: Command::Withdraw { amount: 100 },
        };
        let instruction_data = serialize(&instruction).unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![AccountMeta::new(user_account.pubkey(), false)],
                data: instruction_data,
            }],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        let result = banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
    }
}
