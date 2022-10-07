// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

use crate::StaticSingleAssigner;

use leo_ast::{
    Block, Finalize, Function, FunctionConsumer, Member, Program, ProgramConsumer, ProgramScope, ProgramScopeConsumer,
    StatementConsumer, Struct, StructConsumer,
};
use leo_span::{sym, Symbol};

use indexmap::IndexMap;

impl StructConsumer for StaticSingleAssigner {
    type Output = Struct;

    /// Reconstructs records in the program, ordering its fields such that `owner` and `gates` are the first and second fields, respectively.
    fn consume_struct(&mut self, struct_: Struct) -> Self::Output {
        match struct_.is_record {
            false => struct_,
            true => {
                let mut members = Vec::with_capacity(struct_.members.len());
                let mut member_map: IndexMap<Symbol, Member> = struct_
                    .members
                    .into_iter()
                    .map(|member| (member.identifier.name, member))
                    .collect();

                // Add the owner field to the beginning of the members list.
                // Note that type checking ensures that the owner field exists.
                members.push(member_map.remove(&sym::owner).unwrap());

                // Add the gates field to the beginning of the members list.
                // Note that type checking ensures that the gates field exists.
                members.push(member_map.remove(&sym::gates).unwrap());

                // Add the remaining fields to the members list.
                members.extend(member_map.into_iter().map(|(_, member)| member));

                Struct { members, ..struct_ }
            }
        }
    }
}

impl FunctionConsumer for StaticSingleAssigner {
    type Output = Function;

    /// Reconstructs the `Function`s in the `Program`, while allocating the appropriate `RenameTable`s.
    fn consume_function(&mut self, function: Function) -> Self::Output {
        // Allocate a `RenameTable` for the function.
        self.push();

        // There is no need to reconstruct `function.inputs`.
        // However, for each input, we must add each symbol to the rename table.
        for input_variable in function.input.iter() {
            self.rename_table
                .update(input_variable.identifier().name, input_variable.identifier().name);
        }

        let block = Block {
            span: function.block.span,
            statements: self.consume_block(function.block),
        };

        // Remove the `RenameTable` for the function.
        self.pop();

        let finalize = function.finalize.map(|finalize| {
            // Allocate a `RenameTable` for the finalize block.
            self.push();

            // There is no need to reconstruct `finalize.inputs`.
            // However, for each input, we must add each symbol to the rename table.
            for input_variable in finalize.input.iter() {
                self.rename_table
                    .update(input_variable.identifier().name, input_variable.identifier().name);
            }

            let block = Block {
                span: finalize.block.span,
                statements: self.consume_block(finalize.block),
            };

            // Remove the `RenameTable` for the finalize block.
            self.pop();

            Finalize {
                identifier: finalize.identifier,
                input: finalize.input,
                output: finalize.output,
                output_type: finalize.output_type,
                block,
                span: finalize.span,
            }
        });

        Function {
            annotations: function.annotations,
            call_type: function.call_type,
            identifier: function.identifier,
            input: function.input,
            output: function.output,
            output_type: function.output_type,
            block,
            finalize,
            span: function.span,
        }
    }
}

impl ProgramScopeConsumer for StaticSingleAssigner {
    type Output = ProgramScope;

    fn consume_program_scope(&mut self, input: ProgramScope) -> Self::Output {
        ProgramScope {
            program_id: input.program_id,
            structs: input
                .structs
                .into_iter()
                .map(|(i, s)| (i, self.consume_struct(s)))
                .collect(),
            mappings: input.mappings,
            functions: input
                .functions
                .into_iter()
                .map(|(i, f)| (i, self.consume_function(f)))
                .collect(),
            span: input.span,
        }
    }
}

impl ProgramConsumer for StaticSingleAssigner {
    type Output = Program;

    fn consume_program(&mut self, input: Program) -> Self::Output {
        Program {
            imports: input
                .imports
                .into_iter()
                .map(|(name, import)| (name, self.consume_program(import)))
                .collect(),
            program_scopes: input
                .program_scopes
                .into_iter()
                .map(|(name, scope)| (name, self.consume_program_scope(scope)))
                .collect(),
        }
    }
}
