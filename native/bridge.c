#include <mpfr.h>
#include <stdint.h>
#include <stdlib.h>

struct lc_float {
    mpfr_t value;
};

static mpfr_rnd_t mode(int up) { return up ? MPFR_RNDU : MPFR_RNDD; }

struct lc_float *lc_mp_new(uint32_t bits) {
    struct lc_float *number = malloc(sizeof(*number));
    if (number) mpfr_init2(number->value, bits);
    return number;
}

void lc_mp_drop(struct lc_float *number) {
    mpfr_clear(number->value);
    free(number);
}

static int valid(const struct lc_float *number) {
    return mpfr_number_p(number->value) && !mpfr_overflow_p() && !mpfr_underflow_p();
}

int lc_mp_set_double(struct lc_float *out, double value, int up) {
    mpfr_clear_flags();
    mpfr_set_d(out->value, value, mode(up));
    return valid(out);
}

int lc_mp_set_ratio(struct lc_float *out, const char *value, int up) {
    mpq_t rational;
    mpq_init(rational);
    const int parsed = mpq_set_str(rational, value, 10) == 0 && mpz_sgn(mpq_denref(rational)) != 0;
    mpfr_clear_flags();
    if (parsed) {
        mpq_canonicalize(rational);
        mpfr_set_q(out->value, rational, mode(up));
    }
    mpq_clear(rational);
    return parsed && valid(out);
}

double lc_mp_get(const struct lc_float *number, int up) {
    return mpfr_get_d(number->value, mode(up));
}

int lc_mp_compare(const struct lc_float *a, const struct lc_float *b) {
    return mpfr_cmp(a->value, b->value);
}

int lc_mp_sign(const struct lc_float *a) { return mpfr_sgn(a->value); }

int lc_mp_copy(struct lc_float *out, const struct lc_float *in, int up) {
    mpfr_clear_flags();
    mpfr_set(out->value, in->value, mode(up));
    return valid(out);
}

int lc_mp_binary(struct lc_float *out, const struct lc_float *a, const struct lc_float *b, int op, int up) {
    mpfr_clear_flags();
    switch (op) {
        case 0: mpfr_add(out->value, a->value, b->value, mode(up)); break;
        case 1: mpfr_sub(out->value, a->value, b->value, mode(up)); break;
        case 2: mpfr_mul(out->value, a->value, b->value, mode(up)); break;
        case 3: mpfr_div(out->value, a->value, b->value, mode(up)); break;
        case 4: mpfr_atan2(out->value, a->value, b->value, mode(up)); break;
        default: return 0;
    }
    return valid(out);
}

int lc_mp_unary(struct lc_float *out, const struct lc_float *a, int op, int up) {
    mpfr_clear_flags();
    switch (op) {
        case 0: mpfr_neg(out->value, a->value, mode(up)); break;
        case 1: mpfr_sqrt(out->value, a->value, mode(up)); break;
        case 2: mpfr_exp(out->value, a->value, mode(up)); break;
        case 3: mpfr_log(out->value, a->value, mode(up)); break;
        case 4: mpfr_sin(out->value, a->value, mode(up)); break;
        case 5: mpfr_cos(out->value, a->value, mode(up)); break;
        case 6: mpfr_tan(out->value, a->value, mode(up)); break;
        case 7: mpfr_atan(out->value, a->value, mode(up)); break;
        case 8: mpfr_abs(out->value, a->value, mode(up)); break;
        case 9: mpfr_erf(out->value, a->value, mode(up)); break;
        case 10: mpfr_erfc(out->value, a->value, mode(up)); break;
        default: return 0;
    }
    return valid(out);
}

int lc_mp_pow(struct lc_float *out, const struct lc_float *a, int32_t n, int up) {
    mpfr_clear_flags();
    mpfr_pow_si(out->value, a->value, n, mode(up));
    return valid(out);
}

int lc_mp_root(struct lc_float *out, const struct lc_float *a, uint32_t degree, int up) {
    mpfr_clear_flags();
    mpfr_rootn_ui(out->value, a->value, degree, mode(up));
    return valid(out);
}

int lc_mp_pi(struct lc_float *out, int up) {
    mpfr_clear_flags();
    mpfr_const_pi(out->value, mode(up));
    return valid(out);
}

int lc_mp_has_integer(const struct lc_float *a, const struct lc_float *b) {
    mpfr_t ceiling;
    mpfr_init2(ceiling, mpfr_get_prec(a->value));
    mpfr_ceil(ceiling, a->value);
    const int hit = mpfr_cmp(ceiling, b->value) <= 0;
    mpfr_clear(ceiling);
    return hit;
}

const char *lc_mp_version(void) { return mpfr_get_version(); }
