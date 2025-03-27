ALTER TABLE avs_version_data RENAME COLUMN latest_version_digest TO stable_version_digest;
ALTER TABLE avs_version_data RENAME COLUMN latest_version_tag TO stable_version_tag;
ALTER TABLE avs_version_data
    ADD COLUMN manual_version_tag VARCHAR(255),
    ADD COLUMN manual_version_digest VARCHAR(100),
    ADD COLUMN release_candidate_tag VARCHAR(255),
    ADD COLUMN release_candidate_digest VARCHAR(100);
