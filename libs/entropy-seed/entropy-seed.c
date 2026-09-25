// entropy-seed FILE: credit a seed saved on the previous boot to the kernel pool,
// then replace it with fresh bytes. The Echo has no hwrng and nothing restores
// entropy across boots, so getrandom() (OpenSSL, libsodium, libgcrypt) otherwise
// blocks for 1-5 minutes after boot.
// ponytail: same model as systemd-random-seed; the seed is credited only once
// because it is overwritten right after, and it is only ever written from an
// initialized pool (blocking getrandom), so the first boot still waits once.
#include <fcntl.h>
#include <linux/random.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/random.h>
#include <unistd.h>

#define SEED_BYTES 512
#define URANDOM_MIN_RESEED "/proc/sys/kernel/random/urandom_min_reseed_secs"

static int read_integer(const char *path) {
  char buf[32];
  int fd = open(path, O_RDONLY);
  ssize_t n = fd < 0 ? -1 : read(fd, buf, sizeof(buf) - 1);
  if (fd >= 0) close(fd);
  if (n <= 0) return -1;
  buf[n] = '\0';
  return atoi(buf);
}

static void write_integer(const char *path, int value) {
  char buf[32];
  int fd = open(path, O_WRONLY);
  if (fd < 0) return;
  int n = snprintf(buf, sizeof(buf), "%d\n", value);
  (void)!write(fd, buf, (size_t)n);
  close(fd);
}

int main(int argc, char **argv) {
  struct {
    struct rand_pool_info info;
    unsigned char buf[SEED_BYTES];
  } seed;
  if (argc != 2) {
    fprintf(stderr, "usage: %s FILE\n", argv[0]);
    return 2;
  }
  int fd = open(argv[1], O_RDONLY);
  if (fd >= 0) {
    ssize_t n = read(fd, seed.buf, SEED_BYTES);
    close(fd);
    int rnd = open("/dev/urandom", O_WRONLY);
    seed.info.entropy_count = n > 0 ? (int)n * 8 : 0;
    seed.info.buf_size = n > 0 ? (int)n : 0;
    int old_reseed = read_integer(URANDOM_MIN_RESEED);
    // Linux 3.18 rate-limits a first urandom-to-output-pool transfer for 60 s.
    // Temporarily lift that limit, pull 192 real seed bits, then restore it.
    if (n == SEED_BYTES && rnd >= 0 && old_reseed >= 0 &&
        ioctl(rnd, RNDADDENTROPY, &seed.info) == 0) {
      int ur = open("/dev/urandom", O_RDONLY);
      write_integer(URANDOM_MIN_RESEED, 0);
      int ready = ur >= 0 && read(ur, seed.buf, 24) == 24 &&
                  getrandom(seed.buf, 1, GRND_NONBLOCK) == 1;
      if (ur >= 0) close(ur);
      write_integer(URANDOM_MIN_RESEED, old_reseed);
      fprintf(stderr, "entropy-seed: credited %d bits%s\n", seed.info.entropy_count,
              ready ? " and initialized getrandom" : "");
    } else {
      fprintf(stderr, "entropy-seed: seed not credited\n");
    }
    if (rnd >= 0) close(rnd);
  }
  if (getrandom(seed.buf, SEED_BYTES, 0) != SEED_BYTES) {
    perror("entropy-seed: getrandom");
    return 1;
  }
  char tmp[4096];
  snprintf(tmp, sizeof tmp, "%s.new", argv[1]);
  fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC, 0600);
  if (fd < 0 || write(fd, seed.buf, SEED_BYTES) != SEED_BYTES || fsync(fd) || close(fd) ||
      rename(tmp, argv[1])) {
    perror("entropy-seed: save");
    return 1;
  }
  memset(&seed, 0, sizeof seed);
  return 0;
}
