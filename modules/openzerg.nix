{ config, lib, pkgs, ... }:

let
  cfg = config.services.openzerg;
in
{
  options.services.openzerg = {
    enable = lib.mkEnableOption "OpenZerg Agent";

    agentName = lib.mkOption {
      type = lib.types.str;
      description = "Agent name (unique identifier)";
    };

    managerUrl = lib.mkOption {
      type = lib.types.str;
      default = "ws://10.200.1.1:17531";
      description = "Zerg Swarm Manager WebSocket URL";
    };

    internalToken = lib.mkOption {
      type = lib.types.str;
      description = "Internal token for authentication";
    };

    workspace = lib.mkOption {
      type = lib.types.path;
      default = "/workspace";
      description = "Workspace directory";
    };

    httpPort = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "HTTP server port for file serving";
    };

    package = lib.mkOption {
      type = lib.types.package;
      description = "The openzerg package to use";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.openzerg = {
      description = "OpenZerg Agent - ${cfg.agentName}";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        AGENT_NAME = cfg.agentName;
        MANAGER_URL = cfg.managerUrl;
        INTERNAL_TOKEN = cfg.internalToken;
        WORKSPACE = cfg.workspace;
        HTTP_PORT = toString cfg.httpPort;
        RUST_LOG = "info";
      };

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/openzerg";
        Restart = "always";
        RestartSec = "5s";
        WorkingDirectory = cfg.workspace;
      };
    };

    systemd.tmpfiles.rules = [
      "d ${cfg.workspace} 0755 root root -"
    ];

    networking.firewall.allowedTCPPorts = [ cfg.httpPort ];
  };
}