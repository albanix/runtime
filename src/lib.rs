use std::sync::{
    Mutex,
    Arc
};

use std::collections::{
    VecDeque,
    HashMap
};
use std::pin::Pin;
use std::future::Future;
use std::os::unix::io::RawFd;

use libc::{
    EPOLL_CTL_ADD, EPOLLIN, EPOLLOUT, epoll_event, epoll_wait
};

use std::task::{
    Waker, 
    Wake, 
    Context, 
    Poll
};

pub struct Executor {
    pub queue: Mutex<VecDeque<Arc<Task>>>, // VecDeque - швидше за звичайний Vector і парцює з push_front, push_back. Що є доволі зручним у контексті мого рантайму (vec - 6 O(n), VecDeque O(1)), вона надає FIFO, тому це зручно.
}
pub struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>, // Pin гарантує, що після запінення future не буде переміщена.
    executor: Arc<Executor> // Таска сама мусить себе заплоновуват через Waker, а це вимагає володіння Executor, а не посилання.
}

pub struct Reactor {
    epoll_fd: RawFd,
    wakers: Mutex<HashMap<RawFd, Waker>>
}

impl Executor {
    pub fn spawn<F>(self: &Arc<Self>, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: self.clone(),
        });

        // ставимо таску на виконання в чергу
        self.queue.lock().unwrap().push_back(task);
    }

    pub fn run(&self, reactor: &Reactor) {
        while let Some(task) = self.queue.lock().unwrap().pop_front() {
            let waker = Waker::from(task.clone());
            let mut cx = Context::from_waker(&waker);

            let mut future = task.future.lock().unwrap();

            if let Poll::Pending = future.as_mut().poll(&mut cx) {};
        }

        reactor.wait();
    }
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        let exec = self.executor.clone();
        let mut queue = exec.queue.lock().unwrap();
        queue.push_back(self);
    }
}

impl Reactor {
    pub fn new() -> Self {
        // rawfd індертифікатор ресурса ОС 
        // epoll_fd це лише файловий дескриптор який вкаує на цей kernel-object
        // epoll по факту нічого не знає про задачі він сповіщає про готовність операції.
        let epoll_fd = unsafe {
            libc::epoll_create1(0)
        };
        
        if epoll_fd < 0 {
            panic!("epoll_create: failed!");
        }

        Self {
            epoll_fd, // файловий дескриптор
            wakers: Mutex::new(HashMap::new()), // тут у нас вже є таблица, вона зв'язує fd - яка задча чекає певний fd.
        }
    }

    pub fn register(&self, fd: RawFd, writable: bool, waker: Waker) {
        // event - створюємо структуру epoll_event, events по факту це події які нас цікавлять, тобто запис або читання, u64 це користувацькі дані, тобто epoll поверне тобі назад при epoll_wait()
        let mut event = epoll_event {
            events: if writable { EPOLLOUT as u32 } else { EPOLLIN as u32 },
            u64: fd as u64, 
        };
        
        // можна це описати як: ядро дивився за цим fd, як що він ready, додаємо в таблицю
        unsafe {
            libc::epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, &mut event as *mut _,);
        }
        // коли ядро дає ready ми будимо waker.
        self.wakers.lock().unwrap().insert(fd, waker);
    }

    /// **Блокуємо потік, поки ядро не сповістись про готовність хоча би одного fd.**
    pub fn wait(&self) {
        let mut events = vec![libc::epoll_event { events: 0, u64: 0}; 1024];
        // events - буфер подій

        // epoll - це наш epoll instance, events куда ядро запише події, 1024 макисмум подій за один виклик
        // -1 блокуватись вічно
        let nfds = unsafe {
            epoll_wait(self.epoll_fd, events.as_mut_ptr(), 1024, -1)
        };

        for i in 0..nfds as usize {
            let fd = events[i].u64 as RawFd;
            if let Some(waker)  = self.wakers.lock().unwrap().remove(&fd) {
                waker.wake(); // пробудження задачі
            }
        }

        // оброка подій, достаємо u64 який поклали в register, epoll - не знає, що це fd. Це прості користувацькі дані, ми використовуємо це як ідентифікатор.
    }
}