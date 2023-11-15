pub enum Command {
    Play,
    Pause,
    PlayPause,
    Skip,
    Stop,
    Clear,
    Enqueue(Box<std::path::Path>),
    Dequeue(usize),
}
