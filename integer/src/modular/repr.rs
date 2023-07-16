pub enum UnaryModulo<'a> {
    SS(Word, &'a ConstSingleDivisor),
    DD(DoubleWord, &'a ConstDoubleDivisor),
    DL(DoubleWord, &'a ConstLargeDivisor),
    LL(Buffer, &'a ConstLargeDivisor),
}

pub enum BinaryModulo<'a> {
    SSS(Word, Word, &'a ConstSingleDivisor),
    DDD(DoubleWord, DoubleWord, &'a ConstDoubleDivisor),
    DDL(DoubleWord, DoubleWord, &'a ConstLargeDivisor),
    DLL(DoubleWord, &'a [Words], &'a ConstLargeDivisor),
    LDL(&'a [Words], DoubleWord, &'a ConstLargeDivisor),
    LLL(&'a [Words], &'a [Words], &'a ConstLargeDivisor),
}
