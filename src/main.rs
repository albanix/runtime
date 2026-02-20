use std::sync::{
    Mutex,
    Arc
};
use std::collections::VecDeque;
use std::pin::Pin;
use std::future::Future;
use std::task::{Waker, Wake};

struct Executor {
    queue: Mutex<VecDeque<Arc<Task>>>, // VecDeque - швидше за звичайний Vector і парцює з push_front, push_back. Що є доволі зручним у контексті мого рантайму (vec - 6 O(n), VecDeque O(1)), вона надає FIFO, тому це зручно.
}
struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>, // Pin гарантує, що після запінення future не буде переміщена.
    executor: Arc<Executor> // Таска сама мусить себе заплоновуват через Waker, а це вимагає володіння Executor, а не посилання.
}

impl Executor {
    fn spawn<F>(self: &Arc<Self>, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: self.clone(),
        });

        while let Some(task) = self.queue.lock().unwrap().pop_front() {
            //
        }
    }
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        let exec = self.executor.clone();
        let mut queue = exec.queue.lock().unwrap();
        queue.push_back(self);
    }
}
fn main() {
    print!("123");
}