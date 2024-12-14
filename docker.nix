{ dockerTools
, rss-webhook-trigger
,
}:
dockerTools.buildLayeredImage {
  name = "icewind1991/rss-webhook-trigger";
  tag = "latest";
  maxLayers = 5;
  contents = [
    rss-webhook-trigger
    dockerTools.caCertificates
  ];
  config = {
    Cmd = [ "rss-webhook-trigger" ];
  };
}
