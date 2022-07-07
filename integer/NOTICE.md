Following is the copy of the MIT license of ibig-rs:

```
MIT License

Copyright (c) 2020 Tomek Czajka

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

Initial modifications on the `ibig` library:

1. The underlying represetation of the UBig is vastly changed, the new representation
   supports inline double words and embedded sign bit, and the IBig doesn't support get
   the magnitude as reference now.
2. Operation traits are moved to the `dashu-base` crate.
3. The trait `NextPowerOfTwo` is changed to `PowerOfTwo` with modified definition.
4. Bitwise operators between different signedness are removed to enforce explicitness
5. `AndNot` trait is made private because it's not widely used and the naming doesn't follow the BitXXX style in the std library.