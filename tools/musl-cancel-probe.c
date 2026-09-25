// Probe: real pthread_cancel on the Echo kernel with static musl. Build:
//   armv7l-linux-musleabihf-gcc -O2 -static -pthread musl-cancel-probe.c -o musl-cancel-probe
#include <arpa/inet.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

static pthread_mutex_t m = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cv = PTHREAD_COND_INITIALIZER;
static volatile int cleaned;

static int sock(int type) {
  int fd = socket(AF_INET, type, 0);
  struct sockaddr_in a = {.sin_family = AF_INET, .sin_addr.s_addr = htonl(INADDR_LOOPBACK)};
  if (fd < 0 || bind(fd, (struct sockaddr *)&a, sizeof a)) { perror("socket"); exit(2); }
  if (type == SOCK_STREAM) listen(fd, 1);
  return fd;
}
static void unlock(void *p) { pthread_mutex_unlock(p); cleaned = 1; }
static void mark(void *p) { (void)p; cleaned = 1; }

static void *t_recv(void *p) { char b[16]; pthread_cleanup_push(mark, 0); recv(*(int *)p, b, sizeof b, 0); pthread_cleanup_pop(0); return 0; }
static void *t_accept(void *p) { pthread_cleanup_push(mark, 0); accept(*(int *)p, 0, 0); pthread_cleanup_pop(0); return 0; }
static void *t_usleep(void *p) { (void)p; pthread_cleanup_push(mark, 0); for (;;) usleep(100000); pthread_cleanup_pop(0); return 0; }
static void *t_cond(void *p) {
  (void)p;
  pthread_mutex_lock(&m);
  pthread_cleanup_push(unlock, &m);
  for (;;) pthread_cond_wait(&cv, &m);
  pthread_cleanup_pop(1);
  return 0;
}

static double now_ms(void) {
  struct timespec t;
  clock_gettime(CLOCK_MONOTONIC, &t);
  return t.tv_sec * 1e3 + t.tv_nsec / 1e6;
}

static int run(const char *name, void *(*fn)(void *), void *arg) {
  pthread_t t;
  cleaned = 0;
  pthread_create(&t, 0, fn, arg);
  usleep(200000); // let it block
  double t0 = now_ms();
  pthread_cancel(t);
  void *res;
  pthread_join(t, &res);
  double dt = now_ms() - t0;
  int ok = res == PTHREAD_CANCELED && cleaned && dt < 50;
  printf("%-12s %s  %.2f ms  cleanup=%d\n", name, ok ? "OK  " : "FAIL", dt, cleaned);
  return ok;
}

int main(void) {
  int u = sock(SOCK_DGRAM), l = sock(SOCK_STREAM);
  int ok = run("recv(udp)", t_recv, &u) & run("accept", t_accept, &l) &
           run("usleep", t_usleep, 0) & run("cond_wait", t_cond, 0);
  // mutex must be free again after the cancelled cond_wait cleanup
  int tl = pthread_mutex_trylock(&m);
  ok &= tl == 0;
  // the Echo has no RTC: wall clock may read 2010, so only check the call works
  struct timespec rt;
  ok &= clock_gettime(CLOCK_REALTIME, &rt) == 0;
  printf("realtime=%lld trylock=%d => %s\n", (long long)rt.tv_sec, tl, ok ? "PASS" : "FAIL");
  return !ok;
}
