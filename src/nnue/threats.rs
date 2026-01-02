use crate::{
    lookup::attacks,
    types::{Bitboard, Color, Piece, PieceType, Square},
};

static mut PIECE_PAIR_LOOKUP: [[[u32; 2]; 12]; 12] = [[[0; 2]; 12]; 12];
static mut PIECE_OFFSET_LOOKUP: [[i32; 64]; 12] = [[0; 64]; 12];
static mut ATTACK_INDEX_LOOKUP: [[[u8; 64]; 64]; 12] = [[[0; 64]; 64]; 12];

pub fn initialize() {
    #[rustfmt::skip]
    const PIECE_INTERACTION_MAP: [[i32; 6]; 6] = [
        [0,  1, -1,  2, -1, -1],
        [0,  1,  2,  3,  4,  5],
        [0,  1,  2,  3, -1,  4],
        [0,  1,  2,  3, -1,  4],
        [0,  1,  2,  3,  4,  5],
        [0,  1,  2,  3, -1, -1],
    ];

    const PIECE_TARGET_COUNT: [i32; 6] = [6, 12, 10, 10, 12, 8];

    let mut offset = 0;
    let mut piece_offset = [0; Piece::NUM];
    let mut offset_table = [0; Piece::NUM];

    for piece_color in [Color::White, Color::Black] {
        for piece_type in 0..PieceType::NUM {
            let piece_type = PieceType::new(piece_type);
            let piece = Piece::new(piece_color, piece_type);

            let mut count = 0;

            for (square, entry) in unsafe { PIECE_OFFSET_LOOKUP[piece].iter_mut().enumerate() } {
                *entry = count;

                if piece_type != PieceType::Pawn || (8..56).contains(&square) {
                    count += attacks(piece, Square::new(square as u8), Bitboard(0)).popcount() as i32;
                }
            }

            piece_offset[piece] = count;
            offset_table[piece] = offset;

            offset += PIECE_TARGET_COUNT[piece_type] * count;
        }
    }

    for attacking in Piece::ALL {
        for attacked in Piece::ALL {
            let attacking_piece = attacking.piece_type();
            let attacking_color = attacking.piece_color();

            let attacked_piece = attacked.piece_type();
            let attacked_color = attacked.piece_color();

            let map = PIECE_INTERACTION_MAP[attacking_piece][attacked_piece];
            let base = offset_table[attacking]
                + ((attacked_color as i32) * (PIECE_TARGET_COUNT[attacking_piece] / 2) + map) * piece_offset[attacking];

            let enemy = attacking_color != attacked_color;
            let semi_excluded = attacking_piece == attacked_piece && (enemy || attacking_piece != PieceType::Pawn);
            let excluded = map < 0;

            unsafe { PIECE_PAIR_LOOKUP[attacking][attacked][0] = u32::from(excluded) << 30 | base as u32 };
            unsafe {
                PIECE_PAIR_LOOKUP[attacking][attacked][1] = u32::from(excluded || semi_excluded) << 30 | base as u32
            };
        }
    }

    for piece in Piece::ALL {
        for (from, row) in unsafe { ATTACK_INDEX_LOOKUP[piece].iter_mut().enumerate() } {
            let attacks = attacks(piece, Square::new(from as u8), Bitboard(0));

            for (to, entry) in row.iter_mut().enumerate() {
                *entry = (Bitboard((1u64 << to) - 1) & attacks).popcount() as u8;
            }
        }
    }
}

pub fn threat_index(
    piece: Piece, mut from: Square, attacked: Piece, mut to: Square, mirrored: bool, pov: Color,
) -> u32 {
    let flip = (7 * (mirrored as u8)) ^ (56 * (pov as u8));

    from ^= flip;
    to ^= flip;

    let attacking = Piece::new(Color::new((piece.piece_color() as u8) ^ (pov as u8)), piece.piece_type());
    let attacked = Piece::new(Color::new((attacked.piece_color() as u8) ^ (pov as u8)), attacked.piece_type());

    unsafe {
        PIECE_PAIR_LOOKUP[attacking][attacked][usize::from((from as u8) < (to as u8))]
            + PIECE_OFFSET_LOOKUP[attacking][from] as u32
            + ATTACK_INDEX_LOOKUP[attacking][from][to] as u32
    }
}
