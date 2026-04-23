//! Hashnode GraphQL query strings. Kept as `&'static str` so they ship
//! unchanged to SSR and never bloat the hydrate bundle.

pub const LIST_POSTS: &str = r"
query ListPosts($host: String!, $first: Int!, $after: String) {
  publication(host: $host) {
    posts(first: $first, after: $after) {
      pageInfo { hasNextPage endCursor }
      edges {
        node {
          id title slug brief publishedAt readTimeInMinutes
          coverImage { url }
          tags { name slug }
          author { name username profilePicture }
        }
      }
    }
  }
}";

pub const LIST_POSTS_BY_TAG: &str = r"
query ListPostsByTag($host: String!, $first: Int!, $after: String, $tagSlug: String!) {
  publication(host: $host) {
    posts(first: $first, after: $after, filter: { tagSlugs: [$tagSlug] }) {
      pageInfo { hasNextPage endCursor }
      edges {
        node {
          id title slug brief publishedAt readTimeInMinutes
          coverImage { url }
          tags { name slug }
          author { name username profilePicture }
        }
      }
    }
  }
}";

pub const GET_POST: &str = r"
query GetPost($host: String!, $slug: String!) {
  publication(host: $host) {
    post(slug: $slug) {
      id title slug subtitle brief publishedAt readTimeInMinutes
      coverImage { url }
      tags { name slug }
      author { name username profilePicture }
      content { markdown html }
      seo { title description }
    }
  }
}";

pub const LIST_TAGS: &str = r"
query ListTags($host: String!) {
  publication(host: $host) {
    posts(first: 50) {
      edges { node { tags { name slug } } }
    }
  }
}";

pub const GET_PUBLICATION: &str = r"
query GetPublication($host: String!) {
  publication(host: $host) {
    id title displayTitle
    about { html }
    ogMetaData { image }
    favicon
  }
}";
