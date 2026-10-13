//! Compare the manuscript, the remote revision, and the last copy that matched.

use crate::sync_state::CopyRecord;

/// How the manuscript and one remote have moved since the ledger was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    /// Body hash and remote revision both match the ledger.
    InSync,
    /// The manuscript changed and the remote revision did not.
    LocalAhead,
    /// The remote revision changed and the manuscript did not.
    RemoteAhead,
    /// Both sides changed.
    Diverged,
    /// No ledger entry exists for this link.
    Unsynced,
}

impl Drift {
    /// The word `check` prints.
    pub fn label(self) -> &'static str {
        match self {
            Self::InSync => "in-sync",
            Self::LocalAhead => "local-ahead",
            Self::RemoteAhead => "remote-ahead",
            Self::Diverged => "diverged",
            Self::Unsynced => "unsynced",
        }
    }
}

/// What a copy command is allowed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Write this side and refresh the ledger.
    Write,
    /// The two sides already agree. Do not write.
    Already,
    /// Refuse unless the scribe passed `--force`.
    Refuse,
}

/// Classify drift from the local hash, the remote revision, and the ledger.
pub fn drift(local_hash: &str, remote_revision: &str, record: Option<&CopyRecord>) -> Drift {
    let Some(record) = record else {
        return Drift::Unsynced;
    };
    let local_changed = record.content_sha256 != local_hash;
    let remote_changed = record.revision != remote_revision;
    match (local_changed, remote_changed) {
        (false, false) => Drift::InSync,
        (true, false) => Drift::LocalAhead,
        (false, true) => Drift::RemoteAhead,
        (true, true) => Drift::Diverged,
    }
}

/// Decide a pull. `same_text` means the normalized bodies already match.
pub fn gate_pull(state: Drift, force: bool, local_blank: bool, same_text: bool) -> Gate {
    if force {
        return Gate::Write;
    }
    match state {
        Drift::InSync => Gate::Already,
        Drift::RemoteAhead => Gate::Write,
        Drift::Unsynced if local_blank || same_text => Gate::Write,
        Drift::LocalAhead | Drift::Diverged | Drift::Unsynced => Gate::Refuse,
    }
}

/// Decide a push. `remote_blank` means the remote body has no visible text.
pub fn gate_push(state: Drift, force: bool, remote_blank: bool, same_text: bool) -> Gate {
    if force {
        return Gate::Write;
    }
    match state {
        Drift::InSync => Gate::Already,
        Drift::LocalAhead => Gate::Write,
        Drift::Unsynced if remote_blank || same_text => Gate::Write,
        Drift::RemoteAhead | Drift::Diverged | Drift::Unsynced => Gate::Refuse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(revision: &str, hash: &str) -> CopyRecord {
        CopyRecord {
            revision: revision.into(),
            content_sha256: hash.into(),
        }
    }

    #[test]
    fn the_four_recorded_states() {
        let saved = record("3", "hash-a");
        assert_eq!(drift("hash-a", "3", Some(&saved)), Drift::InSync);
        assert_eq!(drift("hash-b", "3", Some(&saved)), Drift::LocalAhead);
        assert_eq!(drift("hash-a", "4", Some(&saved)), Drift::RemoteAhead);
        assert_eq!(drift("hash-b", "4", Some(&saved)), Drift::Diverged);
        assert_eq!(drift("hash-a", "3", None), Drift::Unsynced);
    }

    #[test]
    fn pull_refuses_local_edits_unless_forced_or_the_page_is_blank() {
        assert_eq!(
            gate_pull(Drift::LocalAhead, false, false, false),
            Gate::Refuse
        );
        assert_eq!(
            gate_pull(Drift::Diverged, false, false, false),
            Gate::Refuse
        );
        assert_eq!(gate_pull(Drift::Diverged, true, false, false), Gate::Write);
        assert_eq!(
            gate_pull(Drift::RemoteAhead, false, false, false),
            Gate::Write
        );
        assert_eq!(gate_pull(Drift::Unsynced, false, true, false), Gate::Write);
        assert_eq!(
            gate_pull(Drift::Unsynced, false, false, false),
            Gate::Refuse
        );
        assert_eq!(gate_pull(Drift::InSync, false, false, true), Gate::Already);
    }

    #[test]
    fn push_refuses_a_moved_remote_unless_forced_or_the_remote_is_blank() {
        assert_eq!(
            gate_push(Drift::RemoteAhead, false, false, false),
            Gate::Refuse
        );
        assert_eq!(
            gate_push(Drift::Diverged, false, false, false),
            Gate::Refuse
        );
        assert_eq!(gate_push(Drift::Diverged, true, false, false), Gate::Write);
        assert_eq!(
            gate_push(Drift::LocalAhead, false, false, false),
            Gate::Write
        );
        assert_eq!(gate_push(Drift::Unsynced, false, true, false), Gate::Write);
        assert_eq!(
            gate_push(Drift::Unsynced, false, false, false),
            Gate::Refuse
        );
        assert_eq!(gate_push(Drift::InSync, false, false, true), Gate::Already);
    }
}
