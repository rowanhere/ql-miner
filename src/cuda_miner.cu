#include <cuda_runtime.h>
#include <stdint.h>
#include <stddef.h>

static __host__ __device__ __forceinline__ uint64_t rotl64(uint64_t x, int n) {
    return (x << n) | (x >> (64 - n));
}

static __host__ __device__ __forceinline__ uint64_t keccak_rc(int round) {
    switch (round) {
        case 0: return 0x0000000000000001ULL;
        case 1: return 0x0000000000008082ULL;
        case 2: return 0x800000000000808aULL;
        case 3: return 0x8000000080008000ULL;
        case 4: return 0x000000000000808bULL;
        case 5: return 0x0000000080000001ULL;
        case 6: return 0x8000000080008081ULL;
        case 7: return 0x8000000000008009ULL;
        case 8: return 0x000000000000008aULL;
        case 9: return 0x0000000000000088ULL;
        case 10: return 0x0000000080008009ULL;
        case 11: return 0x000000008000000aULL;
        case 12: return 0x000000008000808bULL;
        case 13: return 0x800000000000008bULL;
        case 14: return 0x8000000000008089ULL;
        case 15: return 0x8000000000008003ULL;
        case 16: return 0x8000000000008002ULL;
        case 17: return 0x8000000000000080ULL;
        case 18: return 0x000000000000800aULL;
        case 19: return 0x800000008000000aULL;
        case 20: return 0x8000000080008081ULL;
        case 21: return 0x8000000000008080ULL;
        case 22: return 0x0000000080000001ULL;
        default: return 0x8000000080008008ULL;
    }
}

static __host__ __device__ void keccak_f1600(uint64_t s[25]) {
    for (int round = 0; round < 24; ++round) {
        uint64_t c[5], d[5], b[25];

        c[0] = s[0] ^ s[5] ^ s[10] ^ s[15] ^ s[20];
        c[1] = s[1] ^ s[6] ^ s[11] ^ s[16] ^ s[21];
        c[2] = s[2] ^ s[7] ^ s[12] ^ s[17] ^ s[22];
        c[3] = s[3] ^ s[8] ^ s[13] ^ s[18] ^ s[23];
        c[4] = s[4] ^ s[9] ^ s[14] ^ s[19] ^ s[24];

        d[0] = c[4] ^ rotl64(c[1], 1);
        d[1] = c[0] ^ rotl64(c[2], 1);
        d[2] = c[1] ^ rotl64(c[3], 1);
        d[3] = c[2] ^ rotl64(c[4], 1);
        d[4] = c[3] ^ rotl64(c[0], 1);

        for (int i = 0; i < 25; i += 5) {
            s[i + 0] ^= d[0];
            s[i + 1] ^= d[1];
            s[i + 2] ^= d[2];
            s[i + 3] ^= d[3];
            s[i + 4] ^= d[4];
        }

        b[0] = s[0];
        b[1] = rotl64(s[6], 44);
        b[2] = rotl64(s[12], 43);
        b[3] = rotl64(s[18], 21);
        b[4] = rotl64(s[24], 14);
        b[5] = rotl64(s[3], 28);
        b[6] = rotl64(s[9], 20);
        b[7] = rotl64(s[10], 3);
        b[8] = rotl64(s[16], 45);
        b[9] = rotl64(s[22], 61);
        b[10] = rotl64(s[1], 1);
        b[11] = rotl64(s[7], 6);
        b[12] = rotl64(s[13], 25);
        b[13] = rotl64(s[19], 8);
        b[14] = rotl64(s[20], 18);
        b[15] = rotl64(s[4], 27);
        b[16] = rotl64(s[5], 36);
        b[17] = rotl64(s[11], 10);
        b[18] = rotl64(s[17], 15);
        b[19] = rotl64(s[23], 56);
        b[20] = rotl64(s[2], 62);
        b[21] = rotl64(s[8], 55);
        b[22] = rotl64(s[14], 39);
        b[23] = rotl64(s[15], 41);
        b[24] = rotl64(s[21], 2);

        for (int y = 0; y < 5; ++y) {
            int row = y * 5;
            uint64_t x0 = b[row + 0];
            uint64_t x1 = b[row + 1];
            uint64_t x2 = b[row + 2];
            uint64_t x3 = b[row + 3];
            uint64_t x4 = b[row + 4];
            s[row + 0] = x0 ^ ((~x1) & x2);
            s[row + 1] = x1 ^ ((~x2) & x3);
            s[row + 2] = x2 ^ ((~x3) & x4);
            s[row + 3] = x3 ^ ((~x4) & x0);
            s[row + 4] = x4 ^ ((~x0) & x1);
        }

        s[0] ^= keccak_rc(round);
    }
}

static __host__ __device__ __forceinline__ void absorb_byte(uint64_t s[25], uint32_t &pos, uint8_t byte) {
    const uint32_t rate = 136;
    s[pos >> 3] ^= ((uint64_t)byte) << ((pos & 7) * 8);
    ++pos;
    if (pos == rate) {
        keccak_f1600(s);
        pos = 0;
    }
}

static __host__ __device__ void absorb_bytes(uint64_t s[25], uint32_t &pos, const uint8_t *data, size_t len) {
    for (size_t i = 0; i < len; ++i) {
        absorb_byte(s, pos, data[i]);
    }
}

static __host__ __device__ void absorb_u32_le(uint64_t s[25], uint32_t &pos, uint32_t value) {
    absorb_byte(s, pos, (uint8_t)(value));
    absorb_byte(s, pos, (uint8_t)(value >> 8));
    absorb_byte(s, pos, (uint8_t)(value >> 16));
    absorb_byte(s, pos, (uint8_t)(value >> 24));
}

static __host__ __device__ void absorb_u64_le(uint64_t s[25], uint32_t &pos, uint64_t value) {
    for (int i = 0; i < 8; ++i) {
        absorb_byte(s, pos, (uint8_t)(value >> (i * 8)));
    }
}

static __device__ bool has_leading_zero_bits(uint64_t s[25], uint32_t bits) {
    for (uint32_t i = 0; i < bits; ++i) {
        uint32_t byte_index = i >> 3;
        uint32_t bit_in_byte = 7 - (i & 7);
        uint8_t byte = (uint8_t)(s[byte_index >> 3] >> ((byte_index & 7) * 8));
        if (((byte >> bit_in_byte) & 1) != 0) {
            return false;
        }
    }
    return true;
}

static __device__ bool valid_nonce_gpu(
    const uint64_t *prefix_state,
    uint32_t prefix_pos,
    uint64_t nonce,
    uint32_t difficulty_bits
) {
    uint64_t s[25];
    #pragma unroll
    for (int i = 0; i < 25; ++i) {
        s[i] = prefix_state[i];
    }

    uint32_t pos = prefix_pos;
    absorb_u64_le(s, pos, nonce);

    s[pos >> 3] ^= ((uint64_t)0x06) << ((pos & 7) * 8);
    s[(136 - 1) >> 3] ^= ((uint64_t)0x80) << (((136 - 1) & 7) * 8);
    keccak_f1600(s);

    return has_leading_zero_bits(s, difficulty_bits);
}

__global__ void mine_kernel(
    const uint64_t *prefix_state,
    uint32_t prefix_pos,
    uint32_t difficulty_bits,
    uint64_t start_nonce,
    uint64_t total_nonces,
    unsigned long long *found_nonce,
    int *found_flag
) {
    uint64_t index = (uint64_t)blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t stride = (uint64_t)gridDim.x * blockDim.x;

    for (; index < total_nonces; index += stride) {
        if (atomicAdd(found_flag, 0) != 0) {
            return;
        }

        uint64_t nonce = start_nonce + index;
        if (valid_nonce_gpu(
                prefix_state,
                prefix_pos,
                nonce,
                difficulty_bits
            )) {
            if (atomicCAS(found_flag, 0, 1) == 0) {
                *found_nonce = (unsigned long long)nonce;
            }
            return;
        }
    }
}

extern "C" int ql_cuda_mine(
    const uint8_t *previous_hash,
    size_t previous_hash_len,
    const uint8_t *merkle_root,
    size_t merkle_root_len,
    uint64_t block_height,
    const uint8_t *wallet,
    size_t wallet_len,
    uint32_t difficulty_bits,
    uint64_t start_nonce,
    uint64_t total_nonces,
    uint64_t *found_nonce,
    uint64_t *checked,
    int device_id
) {
    if (cudaSetDevice(device_id) != cudaSuccess) {
        return -1;
    }

    uint64_t prefix_state[25];
    uint32_t prefix_pos = 0;
    for (int i = 0; i < 25; ++i) {
        prefix_state[i] = 0;
    }
    absorb_u32_le(prefix_state, prefix_pos, 1);
    absorb_bytes(prefix_state, prefix_pos, previous_hash, previous_hash_len);
    absorb_bytes(prefix_state, prefix_pos, merkle_root, merkle_root_len);
    absorb_u64_le(prefix_state, prefix_pos, block_height);
    absorb_bytes(prefix_state, prefix_pos, wallet, wallet_len);

    uint64_t *d_prefix_state = nullptr;
    unsigned long long *d_found_nonce = nullptr;
    int *d_found_flag = nullptr;

    if (cudaMalloc((void **)&d_prefix_state, sizeof(prefix_state)) != cudaSuccess) return -2;
    if (cudaMalloc((void **)&d_found_nonce, sizeof(unsigned long long)) != cudaSuccess) return -5;
    if (cudaMalloc((void **)&d_found_flag, sizeof(int)) != cudaSuccess) return -6;

    int zero = 0;
    unsigned long long zero_nonce = 0;
    cudaMemcpy(d_prefix_state, prefix_state, sizeof(prefix_state), cudaMemcpyHostToDevice);
    cudaMemcpy(d_found_flag, &zero, sizeof(int), cudaMemcpyHostToDevice);
    cudaMemcpy(d_found_nonce, &zero_nonce, sizeof(unsigned long long), cudaMemcpyHostToDevice);

    const int threads = 256;
    uint64_t blocks64 = (total_nonces + threads - 1) / threads;
    if (blocks64 < 1) blocks64 = 1;
    if (blocks64 > 65535ULL) blocks64 = 65535ULL;
    int blocks = (int)blocks64;

    mine_kernel<<<blocks, threads>>>(
        d_prefix_state,
        prefix_pos,
        difficulty_bits,
        start_nonce,
        total_nonces,
        d_found_nonce,
        d_found_flag
    );

    cudaError_t launch_status = cudaGetLastError();
    if (launch_status != cudaSuccess) {
        cudaFree(d_prefix_state);
        cudaFree(d_found_nonce);
        cudaFree(d_found_flag);
        return -7;
    }

    cudaError_t sync_status = cudaDeviceSynchronize();
    if (sync_status != cudaSuccess) {
        cudaFree(d_prefix_state);
        cudaFree(d_found_nonce);
        cudaFree(d_found_flag);
        return -8;
    }

    int h_found = 0;
    unsigned long long h_nonce = 0;
    cudaMemcpy(&h_found, d_found_flag, sizeof(int), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_nonce, d_found_nonce, sizeof(unsigned long long), cudaMemcpyDeviceToHost);

    cudaFree(d_prefix_state);
    cudaFree(d_found_nonce);
    cudaFree(d_found_flag);

    *checked = total_nonces;
    if (h_found) {
        *found_nonce = (uint64_t)h_nonce;
        return 1;
    }

    return 0;
}
