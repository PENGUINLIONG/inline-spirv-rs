#ifndef COUNTER0
    #define COUNTER0
    #define COUNTER 1
#else
    #ifndef COUNTER1
        #define COUNTER1
        #undef COUNTER
        #define COUNTER 2
    #else
        #ifndef COUNTER2
            #define COUNTER2
            #undef COUNTER
            #define COUNTER 3
        #else

        #endif
    #endif
#endif
