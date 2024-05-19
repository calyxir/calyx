//! Optimize a Calyx program via egglog.

use crate::utils;
use calyx_backend::{Backend, CalyxEggBackend};
use main_error::MainError;

use calyx_ir;
use egglog::{ast::Literal, match_term_app, Term, TermDag};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::{collections::HashMap, rc::Rc};
use tempfile::tempfile;

type Result = std::result::Result<(), MainError>;

fn calyx_to_egglog_str(input: &Path) -> std::result::Result<String, MainError> {
    // Push the rewrite rules.
    let mut program = utils::read_from(utils::RewriteRule::CalyxControl)?;
    let library_path =
        Path::new(option_env!("CALYX_PRIMITIVES_DIR").unwrap_or("../"));
    let ws = calyx_frontend::Workspace::construct(
        &Some(input.to_path_buf()),
        &library_path,
    )
    .unwrap();

    // Convert the Calyx program to egglog.
    let file = tempfile::NamedTempFile::new()?;
    let path = file.into_temp_path();
    let mut output = calyx_utils::OutputFile::File(path.to_path_buf());
    let ctx = calyx_ir::from_ast::ast_to_ir(ws).unwrap();
    CalyxEggBackend::emit(&ctx, &mut output).unwrap();
    let output = fs::read_to_string(path)?;

    // Push the Calyx program post-conversion.
    program.push_str(output.as_str());
    // Push the schedule.
    program.push_str(
        utils::run_schedule(&[utils::RewriteRule::CalyxControl])?.as_str(),
    );
    Ok(program)
}

fn calyx_file_to_egglog(input: &Path, check: &str, display: bool) -> Result {
    let mut program = calyx_to_egglog_str(input)?;
    program.push_str(check);
    if display {
        println!("{}", program);
    }

    let mut egraph = egglog::EGraph::default();
    let result = egraph.parse_and_run_program(&program).map(|lines| {
        for line in lines {
            println!("{}", line);
        }
    });
    if display {
        let serialized = egraph.serialize_for_graphviz(true);
        let file = tempfile::NamedTempFile::new()?;
        let path = file.into_temp_path().with_extension("svg");
        serialized.to_svg_file(path.clone())?;
        std::process::Command::new("open")
            .arg(path.to_str().unwrap())
            .output()?;
    }
    if result.is_err() {
        println!("{:?}", result);
    }
    Ok(result?)
}

fn calyx_to_egglog_debug(input: &str, check: &str) -> Result {
    calyx_to_egglog_internal(input, check, true)
}

fn calyx_to_egglog(input: &str, check: &str) -> Result {
    calyx_to_egglog_internal(input, check, false)
}

fn calyx_to_egglog_internal(input: &str, check: &str, display: bool) -> Result {
    let mut temporary_file = tempfile::NamedTempFile::new()?;
    writeln!(temporary_file, "{}", input)?;
    calyx_file_to_egglog(temporary_file.path(), check, display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calyx_to_egg_simple() -> Result {
        calyx_to_egglog(
            r#"
            import "primitives/core.futil";
            import "primitives/memories/comb.futil";
            
            component main() -> () {
              cells {
                @external(1) in = comb_mem_d1(32, 1, 1);
                a = std_reg(32);
                b = std_reg(32);
              }
            
              wires {
                group A {
                  a.write_en = 1'b1;
                  in.addr0 = 1'b0;
                  a.in = in.read_data;
                  A[done] = a.done;
                }
            
                group B {
                  b.write_en = 1'b1;
                  b.in = a.out;
                  B[done] = b.done;
                }
              }
            
              control {
                seq { @promotable(2) A; @promotable(3) B; }
              }
            }
        "#,
            r#"
            (check (=
                egg-main 
                (Seq (Attributes (map-empty)) 
                (Cons (Enable A (Attributes (map-insert (map-empty) "promotable" 2))) 
                (Cons (Enable B (Attributes (map-insert (map-empty) "promotable" 3)))
                    (Nil))))
            ))"#,
        )
    }

    #[test]
    fn test_calyx_to_egg_compaction() -> Result {
        calyx_to_egglog(
            r#"
    import "primitives/core.futil";
    import "primitives/memories/comb.futil";

    component main () -> () {
      cells {
        a_reg = std_reg(32);
        b_reg = std_reg(32);
        c_reg = std_reg(32);
        d_reg = std_reg(32);
        a = std_add(32);
        ud = undef(1);
      }

      wires {
        group A<"promotable"=1> {
          a_reg.in = 32'd5;
          a_reg.write_en = 1'd1;
          A[done] = a_reg.done;
        }

        group B<"promotable"=10> {
            b_reg.in = 32'd10;
            b_reg.write_en = 1'd1;
            B[done] = ud.out;
          }

        group C<"promotable"=1> {
          a.left = a_reg.out;
          a.right = b_reg.out;
          c_reg.in = a.out;
          c_reg.write_en = 1'd1;
          C[done] = c_reg.done;
        }

        group D<"promotable"=10> {
          d_reg.in = a_reg.out;
          d_reg.write_en = 1'd1;
          D[done] = ud.out;
        }
      }

      control {
        @promotable(22) seq {
          @promotable A;
          @promotable(10) B;
          @promotable C;
          @promotable(10) D;
        }
      }
    }
            "#,
            r#"
                ; seq { A; B; C; D; }
                (check (=
                    egg-main
                    (Seq (Attributes (map-insert (map-empty) "promotable" 22)) 
                        (Cons (Enable A (Attributes (map-insert (map-empty) "promotable" 1))) 
                        (Cons (Enable B (Attributes (map-insert (map-empty) "promotable" 10))) 
                        (Cons (Enable C (Attributes (map-insert (map-empty) "promotable" 1))) 
                        (Cons (Enable D (Attributes (map-insert (map-empty) "promotable" 10))) 
                            (Nil))))))))
                
                ; seq { par { A; } B; C; D; }
                (check (=
                    egg-main
                    (Seq (Attributes (map-insert (map-empty) "promotable" 22)) 
                    (Cons (Par (Attributes (map-empty)) 
                        (Cons (Enable A (Attributes (map-insert (map-empty) "promotable" 1))) 
                            (Nil)
                        )
                    )
                    (Cons (Enable B (Attributes (map-insert (map-empty) "promotable" 10 ))) 
                    (Cons (Enable C (Attributes (map-insert (map-empty) "promotable" 1))) 
                    (Cons (Enable D (Attributes (map-insert (map-empty) "promotable" 10))) 
                        (Nil))))))))
                        
                ; seq { par { A; B; } C; D; }
                (check (=
                    egg-main
                    (Seq (Attributes (map-insert (map-empty) "promotable" 22))
                    (Cons (Par (Attributes (map-empty))
                            (Cons (Enable B (Attributes (map-insert (map-empty) "promotable" 10)))
                            (Cons (Enable A (Attributes (map-insert (map-empty) "promotable" 1)))
                                (Nil))))
                    (Cons (Enable C (Attributes (map-insert (map-empty) "promotable" 1)))
                    (Cons (Enable D (Attributes (map-insert (map-empty) "promotable" 10)))
                        (Nil)))))))
                    

                    "#,
        )
    }
}
