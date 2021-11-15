/// Communicative type alias to represent a termination message received via a termination channel.
pub type TerminationMessage = String;

/// Communicates if a process has received a termination command.
#[derive(Debug, Deserialize, Serialize)]
enum Termination {
    Received,
    Waiting,
}