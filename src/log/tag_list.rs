#![allow(clippy::many_single_char_names)]

use crate::log::tag::Tag;
use crate::log::tag_value::TagValue;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Deref, DerefMut};

/// This struct converts a tuple of tag builders (`Into<Tag>`) to a vector of tags.
/// It supports tuples of length 0 through 20.
#[derive(Clone)]
pub struct TagList(pub Vec<Tag>);
impl TagList {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn append(&mut self, other: &mut TagList) {
        self.0.append(&mut other.0);
    }

    pub fn push(&mut self, name: &'static str, value: impl Into<TagValue>) {
        self.0.push(Tag::new(name, value));
    }

    #[must_use]
    pub fn with(mut self, name: &'static str, value: impl Into<TagValue>) -> Self {
        self.push(name, value);
        self
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<Tag> {
        self.0
    }
}
impl Default for TagList {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for TagList {
    type Target = Vec<Tag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for TagList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Add for TagList {
    type Output = TagList;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.0.extend(rhs.0.into_iter());
        self
    }
}
impl Display for TagList {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Some(tag) = self.0.first() {
            write!(f, "{:?}:{}", tag.name, tag.value)?;
        }
        for tag in self.0.iter().skip(1) {
            write!(f, ",{:?}:{}", tag.name, tag.value)?;
        }
        Ok(())
    }
}
impl Debug for TagList {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "TagList{{")?;
        if let Some(tag) = self.0.first() {
            write!(f, "{:?}:{:?}}}", tag.name, tag.value)?;
        }
        for tag in self.0.iter().skip(1) {
            write!(f, ",{:?}:{:?}", tag.name, tag.value)?;
        }
        write!(f, "}}")
    }
}
impl Eq for TagList {}
impl Hash for TagList {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_slice().hash(state);
    }
}
impl Ord for TagList {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_slice().cmp(other.0.as_slice())
    }
}
impl PartialEq for TagList {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_slice() == other.0.as_slice()
    }
}
impl PartialOrd for TagList {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.as_slice().partial_cmp(other.0.as_slice())
    }
}

impl From<Vec<Tag>> for TagList {
    fn from(v: Vec<Tag>) -> Self {
        Self(v)
    }
}
impl<A: Into<Tag>> From<A> for TagList {
    fn from(a: A) -> Self {
        TagList(vec![a.into()])
    }
}
// From tuples of length 0 through 20.
impl From<()> for TagList {
    fn from(_: ()) -> Self {
        TagList(vec![])
    }
}
impl<A: Into<Tag>> From<(A,)> for TagList {
    fn from((a,): (A,)) -> Self {
        TagList(vec![a.into()])
    }
}
impl<A: Into<Tag>, B: Into<Tag>> From<(A, B)> for TagList {
    fn from((a, b): (A, B)) -> Self {
        TagList(vec![a.into(), b.into()])
    }
}
impl<A: Into<Tag>, B: Into<Tag>, C: Into<Tag>> From<(A, B, C)> for TagList {
    fn from((a, b, c): (A, B, C)) -> Self {
        TagList(vec![a.into(), b.into(), c.into()])
    }
}
impl<A: Into<Tag>, B: Into<Tag>, C: Into<Tag>, D: Into<Tag>> From<(A, B, C, D)> for TagList {
    fn from((a, b, c, d): (A, B, C, D)) -> Self {
        TagList(vec![a.into(), b.into(), c.into(), d.into()])
    }
}
impl<A: Into<Tag>, B: Into<Tag>, C: Into<Tag>, D: Into<Tag>, E: Into<Tag>> From<(A, B, C, D, E)>
    for TagList
{
    fn from((a, b, c, d, e): (A, B, C, D, E)) -> Self {
        TagList(vec![a.into(), b.into(), c.into(), d.into(), e.into()])
    }
}
impl<A: Into<Tag>, B: Into<Tag>, C: Into<Tag>, D: Into<Tag>, E: Into<Tag>, F: Into<Tag>>
    From<(A, B, C, D, E, F)> for TagList
{
    fn from((a, b, c, d, e, f): (A, B, C, D, E, F)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
    > From<(A, B, C, D, E, F, G)> for TagList
{
    fn from((a, b, c, d, e, f, g): (A, B, C, D, E, F, G)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H)> for TagList
{
    fn from((a, b, c, d, e, f, g, h): (A, B, C, D, E, F, G, H)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I)> for TagList
{
    fn from((a, b, c, d, e, f, g, h, i): (A, B, C, D, E, F, G, H, I)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J)> for TagList
{
    fn from((a, b, c, d, e, f, g, h, i, j): (A, B, C, D, E, F, G, H, I, J)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K)> for TagList
{
    fn from((a, b, c, d, e, f, g, h, i, j, k): (A, B, C, D, E, F, G, H, I, J, K)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L)> for TagList
{
    fn from((a, b, c, d, e, f, g, h, i, j, k, l): (A, B, C, D, E, F, G, H, I, J, K, L)) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m): (A, B, C, D, E, F, G, H, I, J, K, L, M),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n): (A, B, C, D, E, F, G, H, I, J, K, L, M, N),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
        O: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o): (
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
        ),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
            o.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
        O: Into<Tag>,
        P: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p): (
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
        ),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
            o.into(),
            p.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
        O: Into<Tag>,
        P: Into<Tag>,
        Q: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q): (
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
        ),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
            o.into(),
            p.into(),
            q.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
        O: Into<Tag>,
        P: Into<Tag>,
        Q: Into<Tag>,
        R: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r): (
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
        ),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
            o.into(),
            p.into(),
            q.into(),
            r.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
        O: Into<Tag>,
        P: Into<Tag>,
        Q: Into<Tag>,
        R: Into<Tag>,
        S: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s): (
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
        ),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
            o.into(),
            p.into(),
            q.into(),
            r.into(),
            s.into(),
        ])
    }
}
impl<
        A: Into<Tag>,
        B: Into<Tag>,
        C: Into<Tag>,
        D: Into<Tag>,
        E: Into<Tag>,
        F: Into<Tag>,
        G: Into<Tag>,
        H: Into<Tag>,
        I: Into<Tag>,
        J: Into<Tag>,
        K: Into<Tag>,
        L: Into<Tag>,
        M: Into<Tag>,
        N: Into<Tag>,
        O: Into<Tag>,
        P: Into<Tag>,
        Q: Into<Tag>,
        R: Into<Tag>,
        S: Into<Tag>,
        T: Into<Tag>,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T)> for TagList
{
    fn from(
        (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t): (
            A,
            B,
            C,
            D,
            E,
            F,
            G,
            H,
            I,
            J,
            K,
            L,
            M,
            N,
            O,
            P,
            Q,
            R,
            S,
            T,
        ),
    ) -> Self {
        TagList(vec![
            a.into(),
            b.into(),
            c.into(),
            d.into(),
            e.into(),
            f.into(),
            g.into(),
            h.into(),
            i.into(),
            j.into(),
            k.into(),
            l.into(),
            m.into(),
            n.into(),
            o.into(),
            p.into(),
            q.into(),
            r.into(),
            s.into(),
            t.into(),
        ])
    }
}
