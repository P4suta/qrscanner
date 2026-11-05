# QR test fixtures

The `alphabet` and `hello` QR matrices in `QRFixtures.swift` are known-answer vectors from ZXing's `EncoderTestCase.testEncode` and `testSimpleUTF8ECI`.
Their decoded payloads are `ABCDEF` and `hello`.
The source is [ZXing EncoderTestCase](https://github.com/zxing/zxing/blob/master/core/src/test/java/com/google/zxing/qrcode/encoder/EncoderTestCase.java).
ZXing is licensed under [Apache License 2.0](https://github.com/zxing/zxing/blob/master/LICENSE).
The fixtures are test inputs and are not included in the application bundle.
