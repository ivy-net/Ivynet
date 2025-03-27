INSERT INTO avs_version_data (
    id,
    node_type,
    chain,
    stable_version_tag,
    stable_version_digest,
    breaking_change_tag,
    breaking_change_datetime
)
VALUES
    (3127, 'altlayer(altlayer-mach)', 'holesky', 'v0.3.0-beta', 'sha256:3a52954348295b05ae706d187ee2e6e7a51c2bb96e58bff57bbcf39cb68802b9', NULL, NULL),
    (3130, 'altlayer-mach(xterio)', 'holesky', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (3132, 'altlayer-mach(cyber)', 'holesky', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (3102, 'altlayer(altlayer-mach)', 'mainnet', 'v0.2.6', 'sha256:5bee0a12d23753964e693fa48ef851343ce480628b7bd5aea1ca57f768be9ec2', NULL, NULL),
    (3105, 'altlayer-mach(xterio)', 'mainnet', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (3107, 'altlayer-mach(cyber)', 'mainnet', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (3103, 'altlayer(gm-network-mach)', 'mainnet', 'v0.2.6', 'sha256:5bee0a12d23753964e693fa48ef851343ce480628b7bd5aea1ca57f768be9ec2', NULL, NULL),
    (1733, 'altlayer(unknown)', 'mainnet', 'v0.2.6', 'sha256:5bee0a12d23753964e693fa48ef851343ce480628b7bd5aea1ca57f768be9ec2', NULL, NULL),
    (3106, 'altlayer-mach(dodo-chain)', 'mainnet', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (1737, 'altlayer-mach(unknown)', 'mainnet', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (3128, 'altlayer(gm-network-mach)', 'holesky', 'v0.3.0-beta', 'sha256:3a52954348295b05ae706d187ee2e6e7a51c2bb96e58bff57bbcf39cb68802b9', NULL, NULL),
    (1754, 'altlayer(unknown)', 'holesky', 'v0.3.0-beta', 'sha256:3a52954348295b05ae706d187ee2e6e7a51c2bb96e58bff57bbcf39cb68802b9', NULL, NULL),
    (3131, 'altlayer-mach(dodo-chain)', 'holesky', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL),
    (1758, 'altlayer-mach(unknown)', 'holesky', 'v0.2.5', 'sha256:d3371423e5689808cc96f7fbfbfbf4523c85ae0d74df1dfec508f40bff967c0d', NULL, NULL);

INSERT INTO avs_version_hash (hash, avs_type, version)
VALUES
    ('sha256:7be88396d02741c493cb642f2d1e95b4c59fe253ea41752be434d5bf878888d3', 'altlayer(altlayer-mach)', 'v0.3.6'),
    ('sha256:df207badf80243f7d289df306ee410d5f6b9ec076f882dba393ed51ab1718033', 'altlayer(gm-network-mach)','v0.3.6'),
    ('sha256:fb86cab9bf02721a59a5466f6f56f186e94ca2f6233405e3e74a94255dbb3d75', 'altlayer(unknown)',       'v0.3.6'),
    ('sha256:cee95501c74c51f7b2c6449f669a8457ee946a5af38df89f002bb4dcbef594c8', 'altlayer-mach(xterio)',   'v0.3.5'),
    ('sha256:0149b8eac63d7d454f8ecb864969f21e8c46ef5621a3812a9651a396419d8a22', 'altlayer-mach(cyber)',    'v0.3.5'),
    ('sha256:679f042cdb2b8c4443856a6587f3671d948822833b536ff31fffe0fc5c2c73e8', 'altlayer-mach(dodo-chain)','v0.3.5'),
    ('sha256:1c5db15ae0c864caf0e54abc14f654acdaa4b872d20811af2ab380a9430e55c7', 'altlayer-mach(unknown)',  'v0.3.5');
