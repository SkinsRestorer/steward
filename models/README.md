# OCR model attribution

The two RTen model files in this directory come from Robert Knight's
[Ocrs pretrained models](https://huggingface.co/robertknight/ocrs):

- `text-detection.rten` was downloaded from the
  [Ocrs model distribution](https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten).
- `text-recognition.rten` was downloaded from the
  [Ocrs model distribution](https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten).

The models are licensed under
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). The model
weights are unmodified. Steward embeds both files into its executable at build
time.

SHA-256 checksums:

```text
f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca  text-detection.rten
e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e  text-recognition.rten
```
