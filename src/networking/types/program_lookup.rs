// Modified for openpacketmon (2026), see NOTICE. Based on `src/networking/types/program_lookup.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::networking::manage_packets::get_local_port;
use crate::networking::types::address_port_pair::AddressPortPair;
use crate::networking::types::data_info::DataInfo;
use crate::networking::types::data_representation::DataRepr;
use crate::networking::types::info_address_port_pair::InfoAddressPortPair;
use crate::networking::types::program::Program;
use listeners::{Process, Protocol};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

const RETRY_TIMEOUT: u128 = 1500; // milliseconds
const VALID_PROGRAM_TIMEOUT: u128 = 60_000; // milliseconds

pub struct ProgramLookup {
    port_tx: Sender<(u16, Protocol)>,
    program_rx: Receiver<(u16, Protocol, Option<Process>)>,
    state: HashMap<(u16, Protocol), LookedUpProgram>,
    programs: HashMap<Program, DataInfo>,
    programs_active_last_interval: HashSet<Program>,
}

impl ProgramLookup {
    pub fn new(
        port_tx: Sender<(u16, Protocol)>,
        program_rx: Receiver<(u16, Protocol, Option<Process>)>,
    ) -> Self {
        Self {
            port_tx,
            program_rx,
            state: HashMap::new(),
            programs: HashMap::new(),
            programs_active_last_interval: HashSet::new(),
        }
    }

    pub fn lookup_and_add_data(
        &mut self,
        key: (u16, Protocol),
        is_new_connection: bool,
        new_data: DataInfo,
    ) -> Program {
        let proc = self.lookup(key, is_new_connection);

        let res = Program::from_proc(proc.as_ref());
        self.programs
            .entry(res.clone())
            .and_modify(|d| d.refresh(new_data))
            .or_insert(new_data);
        self.programs_active_last_interval.insert(res.clone());

        res
    }

    pub fn start_interval(&mut self) {
        for program in self.programs_active_last_interval.drain() {
            if let Some(data) = self.programs.get_mut(&program) {
                data.start_interval();
            }
        }
    }

    pub fn pending_results(&mut self) -> Vec<(u16, Protocol, Option<Process>)> {
        let mut res = Vec::new();
        while let Ok(lookup_res) = self.program_rx.try_recv() {
            res.push(lookup_res);
        }
        res
    }

    fn lookup(&mut self, key: (u16, Protocol), is_new_connection: bool) -> Option<Process> {
        if let Some(looked_up_program) = self.state.get_mut(&key) {
            let program = &looked_up_program.program;
            let was_recently_tried =
                looked_up_program.instant.elapsed().as_millis() < RETRY_TIMEOUT;
            let program_still_valid =
                looked_up_program.instant.elapsed().as_millis() < VALID_PROGRAM_TIMEOUT;
            let already_retried = looked_up_program.retried;

            if program.is_some() && is_new_connection && !program_still_valid {
                looked_up_program.retried = false;
                looked_up_program.instant = Instant::now();
                // send this to the listeners routine
                let _ = self.port_tx.send(key);
                return None;
            }

            if program.is_none() && is_new_connection && !was_recently_tried {
                looked_up_program.retried = program_still_valid;
                looked_up_program.instant = Instant::now();
                // send this to the listeners routine
                let _ = self.port_tx.send(key);
                return None;
            }

            if program.is_none() && !is_new_connection && !was_recently_tried && !already_retried {
                looked_up_program.retried = true;
                looked_up_program.instant = Instant::now();
                // send this to the listeners routine
                let _ = self.port_tx.send(key);
                return None;
            }

            program.clone()
        } else {
            let looked_up_program = LookedUpProgram {
                program: None,
                instant: Instant::now(),
                retried: false,
            };
            self.state.insert(key, looked_up_program);
            // send this to the listeners routine
            let _ = self.port_tx.send(key);
            None
        }
    }

    pub fn update(
        &mut self,
        lookup_res: (u16, Protocol, Option<Process>),
        connections: &mut HashMap<AddressPortPair, InfoAddressPortPair>,
    ) {
        let key = (lookup_res.0, lookup_res.1);
        let proc = lookup_res.2;

        let program = Program::from_proc(proc.as_ref());

        // associate unassigned recent connections on port with the program
        if proc.is_some() {
            let mut reassigned_data = DataInfo::default();
            let mut unknown_data = DataInfo::default();
            for (k, v) in connections.iter_mut() {
                if !v.program.is_unknown() {
                    continue;
                }

                if v.final_instant.elapsed().as_millis() < VALID_PROGRAM_TIMEOUT
                    && get_local_port(k, v.traffic_direction) == Some(key)
                {
                    v.program = program.clone();
                    reassigned_data.refresh(v.data_info());
                } else {
                    unknown_data.refresh(v.data_info());
                }
            }

            if reassigned_data.tot_data(DataRepr::packets(false)) > 0 {
                // assign to known
                self.programs_active_last_interval.insert(program.clone());
                self.programs
                    .entry(program)
                    .and_modify(|d| d.refresh(reassigned_data))
                    .or_insert(reassigned_data);
                // remove from Unknown
                // NOTE: subtracting reassigned_data from Unknown wouldn't correctly reassign final_instant
                self.programs_active_last_interval.insert(Program::Unknown);
                // or_insert not needed: Unknown is already in the map since reassigned data came from it
                self.programs
                    .entry(Program::Unknown)
                    .and_modify(|d| *d = unknown_data);
            }
        }

        // or_insert not needed: key is always inserted in lookup() before being sent to the thread
        self.state.entry(key).and_modify(|looked_up_program| {
            looked_up_program.program = proc;
            looked_up_program.instant = Instant::now();
        });
    }

    pub fn programs(&self) -> &HashMap<Program, DataInfo> {
        &self.programs
    }
}

struct LookedUpProgram {
    program: Option<Process>,
    instant: Instant,
    retried: bool,
}

pub fn lookup_program(
    port_rx: &Receiver<(u16, Protocol)>,
    program_tx: &Sender<(u16, Protocol, Option<Process>)>,
) {
    while let Ok((port, protocol)) = port_rx.recv() {
        let program = listeners::get_process_by_port(port, protocol).ok();
        let _ = program_tx.send((port, protocol, program));
    }
}
