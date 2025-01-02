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

    // Deserialize the instruction data
    let instruction: Instruction =
        deserialize(instruction_data).map_err(|_| ProgramError::InvalidAccountData)?;

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
        _ => return Err(ProgramError::InvalidInstructionData),
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
pub struct Instruction {
    pub command: Command,
}
