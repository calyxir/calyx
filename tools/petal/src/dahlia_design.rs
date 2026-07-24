use crate::adls::{ComponentInfo, PosInfo};
use crate::calyx_timeline::CurrentlyActive;
use crate::design::{Design, GroupId, Stack};
use crate::visuals::flamegraph::{compute_flame, write_flames};
use crate::visuals::perfetto_protos::track_event::Type;
use crate::visuals::timeline::{Timeline, Uuid};
use anyhow::{Ok, Result};
use cranelift_entity::{PrimaryMap, entity_impl};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::fs::File;
use std::path::PathBuf;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default, Ord, PartialOrd)]
pub struct StatementId(u32);
entity_impl!(StatementId, "statement");

#[derive(Clone, Copy, Hash, PartialEq, Eq, Default, Ord, PartialOrd)]
pub struct BlockId(u32);
entity_impl!(BlockId, "block");

#[derive(Debug)]
struct Statement {
    line: String,
    line_num: u64,
    ancestors: Vec<BlockId>,
    parent: Option<BlockId>,
}
impl Statement {
    pub fn display_name(&self) -> String {
        format!("L{:04}: {}", self.line_num, self.line)
    }
}

#[derive(Debug)]
struct Block {
    line: String,
    line_num: u64,
    parent: Option<BlockId>,
}

impl Block {
    pub fn display_name(&self) -> String {
        format!("{}: {}", self.short_name(), self.line)
    }

    pub fn short_name(&self) -> String {
        format!("BL{:04}", self.line_num)
    }
}

/// (This structure follows what we have in `Design` for now)
#[derive(Debug)]
pub struct DahliaDesign {
    blocks: PrimaryMap<BlockId, Block>,
    statements: PrimaryMap<StatementId, Statement>,
    groups_to_statement: FxHashMap<GroupId, StatementId>,
}

impl DahliaDesign {
    pub fn new(
        components: Vec<ComponentInfo>,
        parent_map_filename: Option<String>,
        d: &Design,
    ) -> Result<Self> {
        let mut out = Self {
            blocks: PrimaryMap::new(),
            statements: PrimaryMap::new(),
            groups_to_statement: FxHashMap::default(),
        };
        let group_names_to_ids = d.get_group_name_to_ids();
        let (parent_map, all_block_lines) =
            read_parent_map(parent_map_filename)?;
        let mut all_blocks: FxHashMap<u64, BlockId> = FxHashMap::default();

        // For now, we assume that all Dahlia programs are single-function. This assert will
        // break when a program defies this assumption
        assert_eq!(components.len(), 1);
        let main_component = &components[0];
        assert_eq!(main_component.component, "main");
        // sort by line numbers so that statements with earlier line numbers will be processed first
        let mut linenum_to_init_info: FxHashMap<
            u64,
            (FxHashSet<GroupId>, String),
        > = FxHashMap::default();
        for PosInfo {
            name,
            linenum,
            varname,
            ..
        } in main_component.groups.iter()
        {
            let group_id_set = &group_names_to_ids.get(name).unwrap();
            // Assume that each group will only have one enable in control
            assert_eq!(group_id_set.len(), 1);
            let group_id = group_id_set.iter().next().unwrap();
            let line_contents = varname
                .split("{")
                .next()
                .unwrap()
                .split(";")
                .next()
                .unwrap()
                .to_string();

            let (groups, _line_contents) = linenum_to_init_info
                .entry(*linenum)
                .or_insert((FxHashSet::default(), line_contents));
            groups.insert(*group_id);
        }

        let mut sorted_line_nums =
            (linenum_to_init_info).keys().cloned().collect::<Vec<_>>();
        sorted_line_nums.sort();

        for line_num in sorted_line_nums {
            let (group_ids, line) =
                linenum_to_init_info.get(&line_num).unwrap();

            // we haven't seen this line yet; if it's a block, we will add the block in as well.
            if all_block_lines.contains(&line_num) {
                // a block's immediate parent is always itself, so we want to look for its parent.
                let parent = if let Some(pv) = parent_map.get(&line_num) {
                    if pv.len() > 1 {
                        let parent_line = pv[pv.len() - 2];
                        all_blocks.get(&parent_line).copied()
                    } else {
                        None
                    }
                } else {
                    None
                };
                let b = Block {
                    line_num,
                    line: line.clone(),
                    parent,
                };
                all_blocks.insert(line_num, out.blocks.push(b));
            }
            // compute parent and ancestors list for this statement
            let (parent, ancestors) = if let Some(ancestor_line_nums) =
                parent_map.get(&line_num)
            {
                let ancestors: Vec<BlockId> =
                    ancestor_line_nums.iter().map(|a| all_blocks[a]).collect();
                (ancestors.last().copied(), ancestors)
            } else {
                (None, Vec::new())
            };
            // register the statement
            let s = Statement {
                line_num,
                line: line.clone(),
                ancestors,
                parent,
            };
            let stmt_id = out.statements.push(s);

            for g in group_ids {
                out.groups_to_statement.insert(*g, stmt_id);
            }
        }

        Ok(out)
    }

    pub fn compute_dahlia_trace(
        &self,
        calyx_active: &CurrentlyActive,
    ) -> Result<(Vec<Stack>, DahliaCurrentlyActive)> {
        let mut stack_out: Vec<Stack> = vec![];
        let mut active_this_cycle: DahliaCurrentlyActive =
            DahliaCurrentlyActive::new();
        for active_group in calyx_active.get_active_groups() {
            if let Some(s_id) = self.groups_to_statement.get(active_group) {
                let mut stack: Vec<String> = vec![];
                // the statement in question is active.
                active_this_cycle.add_active_statement(*s_id);
                let statement: &Statement = &self.statements[*s_id];
                for b_id in statement.ancestors.iter() {
                    active_this_cycle.add_active_block(*b_id);
                    let block: &Block = &self.blocks[*b_id];
                    stack.push(block.display_name())
                }
                stack.push(statement.display_name());
                stack_out.push(stack);
            }
        }
        stack_out.sort();
        stack_out.dedup();
        Ok((stack_out, active_this_cycle))
    }
}

#[derive(Clone, Debug, Default)]
pub struct DahliaCurrentlyActive {
    blocks: FxHashSet<BlockId>,
    statements: FxHashSet<StatementId>,
}

impl DahliaCurrentlyActive {
    pub fn new() -> Self {
        Self {
            blocks: FxHashSet::default(),
            statements: FxHashSet::default(),
        }
    }

    pub fn add_active_block(&mut self, block_id: BlockId) {
        self.blocks.insert(block_id);
    }

    pub fn add_active_statement(&mut self, statement_id: StatementId) {
        self.statements.insert(statement_id);
    }

    pub fn resolve(&self, new: &Self) -> Result<(Self, Self)> {
        let ended = Self {
            blocks: self.blocks.difference(&new.blocks).copied().collect(),
            statements: self
                .statements
                .difference(&new.statements)
                .copied()
                .collect(),
        };

        let started = Self {
            blocks: new.blocks.difference(&self.blocks).copied().collect(),
            statements: new
                .statements
                .difference(&self.statements)
                .copied()
                .collect(),
        };

        Ok((started, ended))
    }
}

#[derive(Clone, Debug, Default)]
struct BlockTrackInfo {
    uuid: Uuid,
    name: String,
    short_name: String,
    thread_info: Vec<Uuid>,
    // statement is only in the map only when it's active.
    statement_to_thread: FxHashMap<StatementId, Uuid>,
}

impl BlockTrackInfo {
    pub fn get_thread_for_new_statement(
        &mut self,
        s: StatementId,
        t: &mut Timeline,
    ) -> Result<Uuid> {
        let used_uuids: FxHashSet<Uuid> =
            self.statement_to_thread.values().copied().collect();
        let index = match self
            .thread_info
            .iter()
            .position(|uuid| !used_uuids.contains(uuid))
        {
            Some(i) => i,
            None => self.add_new_thread(t)?,
        };
        let uuid = self.thread_info[index];
        self.statement_to_thread.insert(s, uuid);
        Ok(uuid)
    }

    pub fn get_thread_for_closing_statement(
        &mut self,
        s: &StatementId,
    ) -> Uuid {
        self.statement_to_thread.remove(s).unwrap()
    }
}

impl BlockTrackInfo {
    fn add_new_thread(&mut self, t: &mut Timeline) -> Result<usize> {
        let index = self.thread_info.len();
        let new_thread_name =
            format!("Thread {} ({})", index + 1, self.short_name);
        let uuid = t.register_descriptor(new_thread_name, Some(self.uuid))?;
        self.thread_info.push(uuid);
        Ok(index)
    }
}

#[derive(Clone, Debug, Default)]
struct StatementTrackInfo {
    name: String,
    parent: Option<BlockId>,
    // UUIDs are only created for statements without block parents
    // FIXME: in the future, we should also have a "Thread X" system for statements without block parents as well.
    uuid: Option<Uuid>,
}

#[derive(Debug)]
struct DahliaTimeline {
    timeline: Timeline,
    statement_to_info: FxHashMap<StatementId, StatementTrackInfo>,
    block_to_info: FxHashMap<BlockId, BlockTrackInfo>,
}

impl DahliaTimeline {
    pub fn new(d: &DahliaDesign, out_dir: &str) -> Result<Self> {
        let mut timeline =
            Timeline::new(out_dir, "dahlia_timeline_trace.pftrace")?;
        // create "main" track
        let main_uuid =
            timeline.register_descriptor("main".to_string(), None)?;

        let mut t = Self {
            timeline,
            statement_to_info: FxHashMap::default(),
            block_to_info: FxHashMap::default(),
        };

        // add tracks for blocks
        let mut worklist: VecDeque<(BlockId, &Block)> =
            d.blocks.iter().collect();
        while !worklist.is_empty() {
            let (id, block) = worklist.pop_front().unwrap();
            let parent_uuid = match &block.parent {
                None => main_uuid,
                Some(parent_id) => {
                    if let Some(p) = t.block_to_info.get(parent_id) {
                        p.uuid
                    } else {
                        // we haven't processed the parent yet, so we will come back to this one later
                        worklist.push_back((id, block));
                        continue;
                    }
                }
            };

            let uuid = t
                .timeline
                .register_descriptor(block.display_name(), Some(parent_uuid))?;

            t.block_to_info.insert(
                id,
                BlockTrackInfo {
                    uuid,
                    name: block.display_name(),
                    short_name: block.short_name(),
                    thread_info: vec![],
                    statement_to_thread: FxHashMap::default(),
                },
            );
        }

        for (statement_id, statement) in d.statements.iter() {
            let mut uuid = None;
            let mut parent = None;
            match statement.parent {
                None => {
                    // create a track for this statement specifically
                    let stmt_uuid = t.timeline.register_descriptor(
                        statement.display_name(),
                        Some(main_uuid),
                    )?;
                    uuid = Some(stmt_uuid);
                }
                Some(parent_id) => {
                    parent = Some(parent_id);
                }
            }
            assert!(parent.is_some() ^ uuid.is_some());

            t.statement_to_info.insert(
                statement_id,
                StatementTrackInfo {
                    name: statement.display_name(),
                    parent,
                    uuid,
                },
            );
        }

        Ok(t)
    }

    pub fn update(
        &mut self,
        started: &DahliaCurrentlyActive,
        ended: &DahliaCurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        self.update_helper(ended, cycle_count, Type::SliceEnd)?;

        self.update_helper(started, cycle_count, Type::SliceBegin)?;

        Ok(())
    }

    pub fn output_timeline(self, out_dir: &str) -> Result<()> {
        self.timeline
            .output_timeline(out_dir, "dahlia_timeline_trace.pftrace")
    }

    fn update_helper(
        &mut self,
        diff: &DahliaCurrentlyActive,
        cycle_count: u64,
        event_type: Type,
    ) -> Result<()> {
        for block in diff.blocks.iter() {
            let BlockTrackInfo { uuid, name, .. } =
                self.block_to_info.get(block).unwrap();
            self.timeline.register_event(
                name.clone(),
                *uuid,
                cycle_count,
                event_type,
            );
        }

        let mut sv: Vec<StatementId> =
            diff.statements.iter().copied().collect();
        sv.sort();

        for statement in sv.iter() {
            let StatementTrackInfo { name, parent, uuid } =
                self.statement_to_info.get(statement).unwrap();
            if let Some(uuid) = uuid {
                self.timeline.register_event(
                    name.clone(),
                    *uuid,
                    cycle_count,
                    event_type,
                );
            } else if let Some(parent) = parent {
                // need to find the uuid using the parent
                let parent_block = self.block_to_info.get_mut(parent).unwrap();
                let uuid = match event_type {
                    Type::SliceBegin => parent_block
                        .get_thread_for_new_statement(
                            *statement,
                            &mut self.timeline,
                        )?,
                    Type::SliceEnd => {
                        parent_block.get_thread_for_closing_statement(statement)
                    }
                    _ => panic!("unexpected event type"),
                };
                self.timeline.register_event(
                    name.clone(),
                    uuid,
                    cycle_count,
                    event_type,
                );
            }
        }
        Ok(())
    }
}

pub struct DahliaProfilingInfo {
    design: DahliaDesign,
    timeline: DahliaTimeline,
    currently_active: DahliaCurrentlyActive,
    /// Different Calyx traces can map onto the same Dahlia trace,
    /// so we will go with the most simple option for now (instead of mapping a bitvector to a count)
    trace_info: FxHashMap<Vec<Stack>, u64>,
}

impl DahliaProfilingInfo {
    pub fn new(
        components: Vec<ComponentInfo>,
        parent_file: Option<String>,
        d: &Design,
        out_dir: &str,
    ) -> Result<Self> {
        let design = DahliaDesign::new(components, parent_file, d)?;
        let timeline = DahliaTimeline::new(&design, out_dir)?;
        Ok(Self {
            design,
            timeline,
            currently_active: DahliaCurrentlyActive::default(),
            trace_info: Default::default(),
        })
    }

    pub fn process_cycle(
        &mut self,
        calyx_active: &CurrentlyActive,
        cycle_count: u64,
    ) -> Result<()> {
        let (mut stack, active_this_cycle): (
            Vec<Stack>,
            DahliaCurrentlyActive,
        ) = self.design.compute_dahlia_trace(calyx_active)?;
        if stack.is_empty() {
            stack.push(vec!["Calyx-cycle".to_string()]);
        }
        let curr_count = self.trace_info.entry(stack).or_insert(0);
        *curr_count += 1;

        let (started, ended) =
            self.currently_active.resolve(&active_this_cycle)?;
        self.timeline.update(&started, &ended, cycle_count)?;
        self.currently_active = active_this_cycle;
        Ok(())
    }

    pub fn close(&mut self, total_cycles: u64) -> Result<()> {
        self.timeline.update(
            &DahliaCurrentlyActive::new(),
            &self.currently_active,
            total_cycles,
        )
    }

    pub fn output_flame(&self, out_dir: &str) -> Result<()> {
        let flame_input: Vec<(&u64, &Vec<Stack>)> =
            self.trace_info.iter().map(|(s, c)| (c, s)).collect();
        let flame = compute_flame(flame_input)?;
        let mut scaled_flame = PathBuf::from(out_dir);
        scaled_flame.push("dahlia-scaled-flame.folded");
        let mut flat_flame = PathBuf::from(out_dir);
        flat_flame.push("dahlia-flat-flame.folded");
        write_flames(&flame, Some(scaled_flame), Some(flat_flame))?;
        Ok(())
    }

    pub fn output_timeline(self, out_dir: &str) -> Result<()> {
        self.timeline.output_timeline(out_dir)
    }
}

/// (Parent map, all blocks)
/// where Parent map is a map from statement line numbers to all
/// lines that are block ancestors of the statement (starting from the outermost block).
type ParentInfo = (FxHashMap<u64, Vec<u64>>, FxHashSet<u64>);

fn read_parent_map(parent_map_opt: Option<String>) -> Result<ParentInfo> {
    if let Some(parent_map_filename) = parent_map_opt {
        let parent_map_file = File::open(parent_map_filename)?;
        let parent_map_raw: FxHashMap<String, Vec<u64>> =
            serde_json::from_reader(parent_map_file)?;
        // reverse the parent map s.t.
        let mut all_blocks: FxHashSet<u64> = FxHashSet::default();
        let mut parent_map: FxHashMap<u64, Vec<u64>> = parent_map_raw
            .iter()
            .map(|(k, v)| {
                let k_num: u64 = k.parse().unwrap();
                all_blocks.extend(v.iter().cloned());
                let mut rev = v.clone();
                rev.reverse();
                (k_num, rev)
            })
            .collect();
        for block in all_blocks.iter() {
            let v = parent_map.get_mut(block).unwrap();
            v.push(*block);
        }
        Ok((parent_map, all_blocks))
    } else {
        println!("[Dahlia profiling] Parent map not given!!!");
        Ok((FxHashMap::default(), FxHashSet::default()))
    }
}
