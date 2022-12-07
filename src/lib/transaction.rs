use crate::vm::{self, Identifier, Program, VerifyingKey};
use anyhow::{anyhow, bail, Result};
use log::debug;
use rand;
use serde::{Deserialize, Serialize};
use snarkvm::parameters;
use snarkvm::prelude::FromBytes;
use snarkvm::prelude::{Ciphertext, Record, Testnet3};
use std::fs;
use std::path::Path;
use std::str::FromStr;

// Until we settle on one of the alternatives depending on performance, we have 2 variants for deployment transactions:
// Transaction::Deployment generates verifying keys offline and sends them to the network along with the program
// Transaction::Source just sends the program after being validated, and keys are generated on-chain
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Transaction {
    Deployment {
        id: String,
        deployment: Box<vm::Deployment>,
    },
    Source {
        id: String,
        program: Box<vm::Program>,
    },
    Execution {
        id: String,
        transitions: Vec<vm::Transition>,
    },
}

impl Transaction {
    // Used to generate deployment of a new program in path
    pub fn deployment(path: &Path) -> Result<Self> {
        let program_string = fs::read_to_string(path).unwrap();
        let mut rng = rand::thread_rng();
        debug!("Deploying program {}", program_string);
        let deployment = vm::generate_deployment(&program_string, &mut rng)?;
        let id = uuid::Uuid::new_v4().to_string();
        Ok(Self::Deployment {
            id,
            deployment: Box::new(deployment),
        })
    }

    // Used to generate an execution of a program in path or an execution of the credits program
    pub fn execution(
        path: Option<&Path>,
        function_name: vm::Identifier,
        inputs: &[vm::Value],
        private_key: &vm::PrivateKey,
    ) -> Result<Self> {
        let (program, key) = if let Some(path) = path {
            let program_string = fs::read_to_string(path).unwrap();
            let program: vm::Program =
                snarkvm::prelude::Program::from_str(&program_string).unwrap();
            let verifying_key = get_keys(&program, function_name)?;
            (program, verifying_key)
        } else {
            let verifying_key = get_credits_key(&function_name)?;
            let program = Program::credits()?;
            (program, verifying_key)
        };

        let rng = &mut rand::thread_rng();

        let transitions = vm::execution(program, function_name, inputs, private_key, rng, key)?;

        let id = uuid::Uuid::new_v4().to_string();

        Ok(Self::Execution { id, transitions })
    }

    // Used to generate a deployment without generating the verifying keys locally
    pub fn from_source(path: &Path) -> Result<Self> {
        let program_string = fs::read_to_string(path).unwrap();

        debug!("Deploying non-compiled program {}", program_string);

        let program = vm::generate_program(&program_string)?;

        let id = uuid::Uuid::new_v4().to_string();
        Ok(Transaction::Source {
            id,
            program: Box::new(program),
        })
    }

    pub fn id(&self) -> &str {
        match self {
            Transaction::Deployment { id, .. } => id,
            Transaction::Execution { id, .. } => id,
            Transaction::Source { id, .. } => id,
        }
    }

    pub fn output_records(&self) -> Vec<&Record<Testnet3, Ciphertext<Testnet3>>> {
        if let Transaction::Execution { transitions, .. } = self {
            transitions
                .iter()
                .flat_map(|transition| transition.output_records())
                .map(|(_, record)| record)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// If the transaction is an execution, return the list of input record origins
    /// (in case they are record commitments).
    pub fn origin_commitments(&self) -> Vec<&vm::Field> {
        if let Transaction::Execution {
            ref transitions, ..
        } = self
        {
            transitions
                .iter()
                .flat_map(|transition| transition.origins())
                .filter_map(|origin| {
                    if let vm::Origin::Commitment(commitment) = origin {
                        Some(commitment)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transaction::Deployment { id, deployment } => {
                write!(f, "Deployment({},{})", id, deployment.program_id())
            }
            Transaction::Source { id, program } => {
                write!(f, "Source({},{})", id, program.id())
            }
            Transaction::Execution { id, transitions } => {
                let transition = transitions.first().unwrap();
                let program_id = transition.program_id();
                write!(f, "Execution({program_id},{id})")
            }
        }
    }
}

fn get_credits_key(function_name: &Identifier) -> Result<VerifyingKey> {
    let key = parameters::testnet3::TESTNET3_CREDITS_PROGRAM
        .get(&function_name.to_string())
        .ok_or_else(|| anyhow!("Circuit keys for credits.aleo/{function_name}' not found"))
        .map(|(_, key)| key)?;

    VerifyingKey::from_bytes_le(key)
}

fn get_keys(program: &Program, function_name: Identifier) -> Result<VerifyingKey> {
    let rng = &mut rand::thread_rng();

    if let Some(key) = vm::synthesize_key(program, rng, function_name)? {
        Ok(key)
    } else {
        bail!("verifying key not found for identifier");
    }
}
