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

    llm = {
      baseUrl = lib.mkOption {
        type = lib.types.str;
        default = "https://api.openai.com/v1";
        description = "LLM API base URL (OpenAI/Anthropic compatible)";
      };

      apiKey = lib.mkOption {
        type = lib.types.str;
        description = "LLM API key";
      };

      model = lib.mkOption {
        type = lib.types.str;
        default = "gpt-4o";
        description = "LLM model name";
      };
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
        LLM_BASE_URL = cfg.llm.baseUrl;
        LLM_API_KEY = cfg.llm.apiKey;
        LLM_MODEL = cfg.llm.model;
        RUST_LOG = "info";
      };

      path = with pkgs; [
        git
        bash
        systemd
      ];

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
      "d /run/openzerg 0755 root root -"
      "d /run/openzerg/processes 0755 root root -"
    ];
  };
}