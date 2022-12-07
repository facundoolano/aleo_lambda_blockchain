use super::Program;
use anyhow::{ensure, Result};
use snarkvm::prelude::{RegisterTypes, Testnet3, UniversalSRS};
/// This module includes helper functions initially taken from SnarkVM's Stack struct.
/// The goal is to progressively remove the dependency on that struct.
use std::sync::Arc;

pub type Stack = snarkvm::prelude::Stack<Testnet3>;

/// This function creates and initializes a `Stack` struct for a given program on the fly, providing functionality
/// related to Programs (deploy, executions, key synthesis) without the need of a `Process`. It essentially combines
/// Stack::new() and Stack::init()
pub fn new_init(program: &Program) -> Result<Stack> {
    // Retrieve the program ID.
    let program_id = program.id();

    // Ensure the program contains functions.
    ensure!(
        !program.functions().is_empty(),
        "No functions present in the deployment for program '{program_id}'"
    );

    // Construct the stack for the program.
    let mut stack = new_stack(program)?;

    // Add the program functions to the stack.
    add_functions_to_stack(&mut stack, program)?;

    // Return the stack.
    Ok(stack)
}

fn add_functions_to_stack(stack: &mut Stack, program: &Program) -> Result<(), anyhow::Error> {
    for function in program.functions().values() {
        let name = function.name();
        // Ensure the function name is not already added.
        ensure!(
            !stack.register_types.contains_key(name),
            "Function '{name}' already exists"
        );

        // Compute the register types.
        let register_types = RegisterTypes::from_function(&*stack, function)?;
        // Add the function name and register types to the stack.
        stack.register_types.insert(*name, register_types);
    }

    Ok(())
}

fn new_stack(program: &Program) -> Result<Stack> {
    let universal_srs = Arc::new(UniversalSRS::<Testnet3>::load()?);

    Ok(Stack {
        program: program.clone(),
        external_stacks: Default::default(),
        register_types: Default::default(),
        finalize_types: Default::default(),
        universal_srs,
        proving_keys: Default::default(),
        verifying_keys: Default::default(),
    })
}
