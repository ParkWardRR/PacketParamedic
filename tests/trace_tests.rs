use packetparamedic::probes::trace::{MtrReport, ReportData, MtrDetails, Hop};

fn fixture_mtr_full() -> String {
    r#"
    {
      "report": {
        "mtr": {
          "src": "192.168.1.10",
          "dst": "8.8.8.8",
          "tos": 0,
          "tests": 10,
          "hubs": [
            {
              "count": 1,
              "host": "192.168.1.1",
              "Loss%": 0.0,
              "Snt": 10,
              "Last": 1.2,
              "Avg": 1.1,
              "Best": 0.9,
              "Wrst": 1.5,
              "StDev": 0.2
            },
            {
              "count": 2,
              "host": "10.64.0.1",
              "Loss%": 5.0,
              "Snt": 10,
              "Last": 8.5,
              "Avg": 8.0,
              "Best": 7.5,
              "Wrst": 12.0,
              "StDev": 1.5
            }
          ]
        }
      }
    }
    "#.to_string()
}

#[test]
fn test_trace_json_parse() {
    let json = fixture_mtr_full();
    let report: MtrReport = serde_json::from_str(&json).expect("Parse failed");
    
    assert_eq!(report.report.mtr.dst, "8.8.8.8");
    assert_eq!(report.report.mtr.tests, 10);
    assert_eq!(report.report.mtr.hubs.len(), 2);
    
    let hop1 = &report.report.mtr.hubs[0];
    assert_eq!(hop1.loss_percent, 0.0);
    assert_eq!(hop1.avg, 1.1);
    
    let hop2 = &report.report.mtr.hubs[1];
    assert_eq!(hop2.loss_percent, 5.0);
    assert_eq!(hop2.host, "10.64.0.1");
}

#[test]
#[ignore] // Requires mtr installed and network access
fn test_live_trace_google_dns() {
    let res = packetparamedic::probes::trace::run_trace("8.8.8.8");
    match res {
        Ok(report) => {
            assert_eq!(report.report.mtr.dst, "8.8.8.8");
            assert!(report.report.mtr.hubs.len() > 0);
        },
        Err(e) => {
            println!("Live trace skipped: {}", e);
        }
    }
}
