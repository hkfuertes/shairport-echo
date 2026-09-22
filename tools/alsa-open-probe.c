#include <alsa/asoundlib.h>
#include <stdio.h>

static int fail(const char *step, int error) {
  fprintf(stderr, "%s: %s (%d)\n", step, snd_strerror(error), error);
  return 1;
}

int main(int argc, char **argv) {
  const char *device = argc > 1 ? argv[1] : "hw:0,23";
  snd_pcm_t *pcm = NULL;
  snd_pcm_hw_params_t *params;
  unsigned int rate = 48000;
  int direction = 0;
  snd_pcm_uframes_t period_frames = 1024;
  snd_pcm_uframes_t buffer_frames = 4096;

  int error = snd_pcm_open(&pcm, device, SND_PCM_STREAM_PLAYBACK, SND_PCM_NONBLOCK);
  if (error < 0)
    return fail("snd_pcm_open", error);

  snd_pcm_hw_params_alloca(&params);
  if ((error = snd_pcm_hw_params_any(pcm, params)) < 0 ||
      (error = snd_pcm_hw_params_set_access(pcm, params, SND_PCM_ACCESS_RW_INTERLEAVED)) < 0 ||
      (error = snd_pcm_hw_params_set_format(pcm, params, SND_PCM_FORMAT_S16_LE)) < 0 ||
      (error = snd_pcm_hw_params_set_rate_near(pcm, params, &rate, &direction)) < 0 ||
      (error = snd_pcm_hw_params_set_channels(pcm, params, 2)) < 0 ||
      (error = snd_pcm_hw_params_set_period_size_near(pcm, params, &period_frames, &direction)) <
          0 ||
      (error = snd_pcm_hw_params_set_buffer_size_near(pcm, params, &buffer_frames)) < 0 ||
      (error = snd_pcm_hw_params(pcm, params)) < 0) {
    snd_pcm_close(pcm);
    return fail("snd_pcm_hw_params", error);
  }

  snd_pcm_hw_params_get_rate(params, &rate, &direction);
  snd_pcm_hw_params_get_period_size(params, &period_frames, &direction);
  snd_pcm_hw_params_get_buffer_size(params, &buffer_frames);
  printf("opened %s: S16_LE, %u Hz, 2 channels, period %lu, buffer %lu frames\n", device, rate,
         (unsigned long)period_frames, (unsigned long)buffer_frames);

  if ((error = snd_pcm_close(pcm)) < 0)
    return fail("snd_pcm_close", error);
  return 0;
}
