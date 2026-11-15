//! Pruning timeline for the front-end (child module of `backend`): live cuts, undo, rewinding,
//! and the "what if" control tree computed on a worker thread.
//!
//! * Checkpoints: a copy of the simulation at the start of every day on which a cut was made, before
//!   that day's first cut. Undo restores the checkpoint of the last cut's day and replays the log
//!   without that cut up to the present; rewinding restores the latest checkpoint before the target.
//! * Control tree: the same seed and log without cuts `>= n` (n = 0: never pruned), started from the
//!   checkpoint of cut `n` and advanced to the live day in the background (never on the frame).

use super::Backend;
use dendro_core::{Action, LogEvent, MeshData, Simulation};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Most checkpoints kept (older ones are thinned out; a rewind falls back to an earlier one). A
/// 200-year oak checkpoint is ~8 MB.
const MAX_CHECKPOINTS: usize = 12;

struct Worker {
    handle: JoinHandle<Option<(Simulation, Option<MeshData>)>>,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct Timeline {
    /// Fresh simulation (day 0) of the current seed / site.
    base: Option<Simulation>,
    checkpoints: Vec<Simulation>,
    /// Cuts kept by the control tree.
    control_cuts: usize,
    control: Option<Simulation>,
    control_mesh: Option<MeshData>,
    control_mesh_day: u64,
    worker: Option<Worker>,
    /// Bumped whenever the control result is replaced.
    pub control_version: i64,
}

impl Drop for Timeline {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

impl Timeline {
    fn stop_worker(&mut self) {
        if let Some(w) = self.worker.take() {
            w.cancel.store(true, Ordering::Relaxed);
            let _ = w.handle.join();
        }
    }
}

fn cut_count(log: &[LogEvent]) -> usize {
    log.iter().filter(|e| matches!(e.action, Action::Cut(_))).count()
}

/// The log without cuts number `n` and later (drought toggles are kept).
fn without_cuts_from(log: &[LogEvent], n: usize) -> Vec<LogEvent> {
    let mut k = 0;
    log.iter()
        .filter(|e| match e.action {
            Action::Cut(_) => {
                k += 1;
                k <= n
            }
            _ => true,
        })
        .copied()
        .collect()
}

impl Backend {
    fn timeline_base(&mut self) -> Simulation {
        if self.timeline.base.is_none() {
            // Only valid before anything happened: rebuild the fresh simulation of this site.
            let fresh = Backend::with_site(self.sim.seed(), self.soil, self.species).sim;
            self.timeline.base = Some(fresh);
        }
        self.timeline.base.clone().expect("base set")
    }

    /// Called right after a reset (fresh simulation).
    pub(crate) fn timeline_reset(&mut self) {
        self.timeline.stop_worker();
        self.timeline = Timeline::default();
        self.timeline.base = Some(self.sim.checkpoint());
    }

    fn checkpoint_today(&mut self) {
        let day = self.sim.days_elapsed();
        if self.timeline.checkpoints.last().is_some_and(|c| c.days_elapsed() == day) {
            return;
        }
        if self.timeline.base.is_none() {
            self.timeline_base();
        }
        self.timeline.checkpoints.push(self.sim.checkpoint());
        if self.timeline.checkpoints.len() > MAX_CHECKPOINTS {
            // Thin out the older half.
            let n = self.timeline.checkpoints.len();
            let mut k = 0;
            self.timeline.checkpoints.retain(|_| {
                k += 1;
                k > n / 2 || k % 2 == 0
            });
        }
    }

    /// Latest saved state at or before `day`.
    fn state_before(&mut self, day: u64) -> Simulation {
        match self.timeline.checkpoints.iter().rev().find(|c| c.days_elapsed() <= day) {
            Some(c) => c.clone(),
            None => self.timeline_base(),
        }
    }

    fn adopt(&mut self, sim: Simulation) {
        self.sim = sim;
        self.carry_days = 0.0;
        self.drought = self.sim.drought();
    }

    /// Cut now; returns the cut type code, -1 when the node does not exist.
    pub fn prune_cut(&mut self, node_id: u32, t: f32, flush: bool) -> i32 {
        if self.sim.cut_preview(node_id, t, flush).is_none() {
            return -1;
        }
        self.checkpoint_today();
        self.drop_future_checkpoints();
        self.sim.cut(node_id, t, flush).map_or(-1, |k| k.code() as i32)
    }

    /// A rewind left later events in the log: a new cut discards them, and the checkpoints after today.
    fn drop_future_checkpoints(&mut self) {
        let day = self.sim.days_elapsed();
        self.timeline.checkpoints.retain(|c| c.days_elapsed() <= day);
    }

    /// High-level operation: "raise" (height m), "top" (height m), "coppice", "pollard" (recut the
    /// heads), "thin" (fraction), "reduce" (fraction). Returns the number of cuts.
    pub fn prune_op(&mut self, op: &str, arg: f32) -> i32 {
        self.checkpoint_today();
        self.drop_future_checkpoints();
        let n = match op {
            "raise" => self.sim.raise_crown(arg),
            "top" => self.sim.top_at(arg).map_or(0, |_| 1),
            "coppice" => self.sim.coppice().map_or(0, |_| 1),
            "pollard" => self.sim.pollard_recut(),
            "thin" => self.sim.thin_crown(arg),
            "reduce" => self.sim.reduce_crown(arg),
            _ => 0,
        };
        if n == 0 {
            // Nothing cut: forget the checkpoint taken for it.
            let day = self.sim.days_elapsed();
            if !self.sim.prune_log().iter().any(|e| e.day == day && matches!(e.action, Action::Cut(_))) {
                self.timeline.checkpoints.retain(|c| c.days_elapsed() != day);
            }
        }
        n as i32
    }

    /// Undo the last cut (re-simulates from its day to now). False when there is none.
    pub fn prune_undo(&mut self) -> bool {
        let log = self.sim.prune_log().to_vec();
        let now = self.sim.days_elapsed();
        // The last cut already made (after a rewind, later cuts are still in the log, pending).
        let Some(k) = log.iter().rposition(|e| e.day <= now && matches!(e.action, Action::Cut(_))) else { return false };
        let cut_day = log[k].day;
        let mut new_log = log;
        new_log.remove(k);
        let start = self.state_before(cut_day);
        let sim = Simulation::replayed(&start, &new_log, now.max(start.days_elapsed()));
        self.adopt(sim);
        if !new_log.iter().any(|e| e.day == cut_day && matches!(e.action, Action::Cut(_))) {
            self.timeline.checkpoints.retain(|c| c.days_elapsed() != cut_day);
        }
        let cuts = cut_count(&new_log);
        if self.timeline.control_cuts > cuts {
            self.control_set_cuts(cuts);
        }
        true
    }

    /// Jump to `day` (days since germination), backwards or forwards, keeping the log: later cuts
    /// are re-applied when their day comes again (a new cut discards them).
    pub fn prune_seek_day(&mut self, day: u64) {
        let now = self.sim.days_elapsed();
        if day >= now {
            self.sim.advance_days((day - now) as u32);
            return;
        }
        let log = self.sim.prune_log().to_vec();
        let start = self.state_before(day);
        let sim = Simulation::replayed(&start, &log, day);
        self.adopt(sim);
    }

    pub fn prune_cut_count(&self) -> usize {
        cut_count(self.sim.prune_log())
    }

    // --------------------------------------------------------------------------------------------
    // Control tree
    // --------------------------------------------------------------------------------------------

    /// The control keeps the first `n` cuts (0 = never pruned).
    pub fn control_set_cuts(&mut self, n: usize) {
        let n = n.min(self.prune_cut_count());
        if n != self.timeline.control_cuts || self.timeline.control.is_none() {
            self.timeline.stop_worker();
            self.timeline.control_cuts = n;
            self.timeline.control = None;
            self.timeline.control_mesh = None;
            self.timeline.control_version += 1;
        }
    }

    pub fn control_cuts(&self) -> usize {
        self.timeline.control_cuts
    }

    /// Advance the control tree towards the live day in the background; call every frame while the
    /// comparison is shown. Returns true when the control is up to date with the live tree.
    pub fn control_poll(&mut self, want_mesh: bool) -> bool {
        let live_day = self.sim.days_elapsed();
        let log = self.sim.prune_log().to_vec();
        if cut_count(&log) <= self.timeline.control_cuts {
            // No cut after the ones the control keeps: the control is the live tree itself.
            return true;
        }
        if let Some(w) = &self.timeline.worker {
            if !w.handle.is_finished() {
                return false;
            }
            let w = self.timeline.worker.take().expect("worker");
            if let Ok(Some((sim, mesh))) = w.handle.join() {
                if let Some(m) = mesh {
                    self.timeline.control_mesh = Some(m);
                    self.timeline.control_mesh_day = sim.days_elapsed();
                }
                self.timeline.control = Some(sim);
                self.timeline.control_version += 1;
            }
        }
        // (Re)start from the checkpoint of the first dropped cut when there is no usable control.
        let usable = self.timeline.control.as_ref().is_some_and(|c| c.days_elapsed() <= live_day);
        if !usable {
            let n = self.timeline.control_cuts;
            let mut k = 0;
            let first_dropped = log.iter().find(|e| {
                if matches!(e.action, Action::Cut(_)) {
                    k += 1;
                    k > n
                } else {
                    false
                }
            });
            let day = first_dropped.map_or(live_day, |e| e.day);
            let start = self.state_before(day);
            self.timeline.control = Some(start);
        }
        let c = self.timeline.control.as_ref().expect("control");
        let need_mesh = want_mesh && (self.timeline.control_mesh.is_none() || self.timeline.control_mesh_day != live_day);
        if c.days_elapsed() == live_day && !need_mesh {
            return true;
        }
        let mut sim = self.timeline.control.take().expect("control");
        let ctl_log = without_cuts_from(&log, self.timeline.control_cuts);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let handle = std::thread::spawn(move || {
            sim.set_prune_log(ctl_log);
            while sim.days_elapsed() < live_day {
                if flag.load(Ordering::Relaxed) {
                    return None;
                }
                let n = (live_day - sim.days_elapsed()).min(30) as u32;
                sim.advance_days(n);
            }
            let mesh = want_mesh.then(|| sim.wood_mesh());
            Some((sim, mesh))
        });
        self.timeline.worker = Some(Worker { handle, cancel });
        false
    }

    /// The control tree's metrics, if computed (None while the control is the live tree or pending).
    pub fn control_sim(&self) -> Option<&Simulation> {
        if self.prune_cut_count() <= self.timeline.control_cuts {
            return Some(&self.sim);
        }
        self.timeline.control.as_ref()
    }

    pub fn control_mesh(&self) -> Option<&MeshData> {
        if self.prune_cut_count() <= self.timeline.control_cuts {
            return None;
        }
        self.timeline.control_mesh.as_ref()
    }
}

impl Backend {
    /// The live simulation (read-only: pruning queries).
    pub fn live(&self) -> &Simulation {
        &self.sim
    }
}

impl Backend {
    pub fn control_version(&self) -> i64 {
        self.timeline.control_version
    }
}
