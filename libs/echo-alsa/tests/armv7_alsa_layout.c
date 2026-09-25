#include <stddef.h>
#include <sys/ioctl.h>
#include <sound/asound.h>

_Static_assert(sizeof(struct snd_pcm_hw_params) == 604, "snd_pcm_hw_params");
_Static_assert(sizeof(struct snd_pcm_sw_params) == 104, "snd_pcm_sw_params");
_Static_assert(offsetof(struct snd_pcm_sw_params, period_step) == 4, "sw period_step");
_Static_assert(offsetof(struct snd_pcm_sw_params, avail_min) == 12, "sw avail_min");
_Static_assert(offsetof(struct snd_pcm_sw_params, xfer_align) == 16, "sw xfer_align");
_Static_assert(offsetof(struct snd_pcm_sw_params, start_threshold) == 20, "sw start_threshold");
_Static_assert(offsetof(struct snd_pcm_sw_params, stop_threshold) == 24, "sw stop_threshold");
_Static_assert(offsetof(struct snd_pcm_sw_params, boundary) == 36, "sw boundary");
_Static_assert(sizeof(struct snd_xferi) == 12, "snd_xferi");
_Static_assert(sizeof(struct snd_ctl_elem_id) == 64, "snd_ctl_elem_id");
_Static_assert(sizeof(struct snd_ctl_elem_list) == 72, "snd_ctl_elem_list");
_Static_assert(sizeof(struct snd_ctl_elem_info) == 272, "snd_ctl_elem_info");
_Static_assert(sizeof(struct snd_ctl_elem_value) == 712, "snd_ctl_elem_value");
_Static_assert(_Alignof(struct snd_ctl_elem_value) == 8, "snd_ctl_elem_value alignment");
_Static_assert(sizeof(struct snd_ctl_event) == 72, "snd_ctl_event");
_Static_assert(offsetof(struct snd_ctl_event, type) == 0, "ctl event type");
_Static_assert(offsetof(struct snd_ctl_event, data.elem.mask) == 4, "ctl event mask");
_Static_assert(offsetof(struct snd_ctl_event, data.elem.id.numid) == 8, "ctl event numid");
_Static_assert(sizeof(struct snd_ctl_tlv) == 8, "snd_ctl_tlv header");
_Static_assert(sizeof(long) == 4, "ARMv7 kernel long");
_Static_assert(offsetof(struct snd_ctl_elem_value, value.integer.value) == 72, "ctl integer values");
_Static_assert(offsetof(struct snd_ctl_elem_info, value.integer.min) == 80, "ctl integer min");
_Static_assert(offsetof(struct snd_ctl_elem_info, value.integer.max) == 84, "ctl integer max");

_Static_assert(offsetof(struct snd_pcm_hw_params, masks) == 4, "hw masks");
_Static_assert(offsetof(struct snd_pcm_hw_params, intervals) == 260, "hw intervals");
_Static_assert(offsetof(struct snd_pcm_hw_params, rmask) == 512, "hw rmask");
_Static_assert(offsetof(struct snd_pcm_hw_params, fifo_size) == 536, "hw fifo_size");
_Static_assert(offsetof(struct snd_pcm_hw_params, reserved) == 540, "hw reserved");
_Static_assert(offsetof(struct snd_xferi, result) == 0, "xfer result");
_Static_assert(offsetof(struct snd_xferi, buf) == 4, "xfer buf");
_Static_assert(offsetof(struct snd_xferi, frames) == 8, "xfer frames");

_Static_assert(SNDRV_PCM_IOCTL_HW_PARAMS == 0xc25c4111U, "hw params ioctl");
_Static_assert(SNDRV_PCM_IOCTL_SW_PARAMS == 0xc0684113U, "sw params ioctl");
_Static_assert(SNDRV_PCM_IOCTL_PREPARE == 0x00004140U, "prepare ioctl");
_Static_assert(SNDRV_PCM_IOCTL_DELAY == 0x80044121U, "delay ioctl");
_Static_assert(SNDRV_PCM_IOCTL_WRITEI_FRAMES == 0x400c4150U, "writei ioctl");
_Static_assert(SNDRV_CTL_IOCTL_ELEM_LIST == 0xc0485510U, "elem list ioctl");
_Static_assert(SNDRV_CTL_IOCTL_ELEM_INFO == 0xc1105511U, "elem info ioctl");
_Static_assert(SNDRV_CTL_IOCTL_ELEM_WRITE == 0xc2c85513U, "elem write ioctl");
_Static_assert(SNDRV_CTL_IOCTL_SUBSCRIBE_EVENTS == 0xc0045516U, "subscribe events ioctl");
_Static_assert(SNDRV_CTL_IOCTL_TLV_READ == 0xc008551aU, "TLV read ioctl");

int main(void) { return 0; }
