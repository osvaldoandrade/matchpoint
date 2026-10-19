# Primary results outside application storage

The user selects a launch directory or explicit output destination. The asset is
the integrity/confidentiality of original output keys and unrelated local files.
A local process may create a destination while matching is in progress. Job
metadata remains in owner-only application storage; output destinations are
absolute and pinned at preparation rather than reinterpreted during resume.

Existing destinations, including dangling symlinks, are rejected at preparation.
Atomic hard-link publication cannot replace a concurrently created target.
Across filesystems, copying uses an exclusive temporary file in the target
directory, owner-only permissions, sync and the same no-overwrite publication.
Failed copies remove only their own temporary file. Partial matching output
remains private and resumable. Checksums verify the actual published file.

Exporting a verified primary result to itself is a no-op. Job deletion removes
owned application state and preserves external primary/output files. A modified
completed file fails verification and is never silently overwritten. No secret,
identifier payload, network boundary or crypto policy changes. Protection from
a privileged local process or user-controlled parent-directory replacement is
outside the existing local filesystem trust model.
