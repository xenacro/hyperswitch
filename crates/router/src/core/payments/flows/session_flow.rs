use async_trait::async_trait;

use super::{ConstructFlowSpecificData, DecideFlow, Feature, Flow};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, storage, PaymentsSessionData, PaymentsSessionResponseData},
};

impl Flow<api::Session, types::PaymentsSessionData, types::PaymentsSessionResponseData>
    for PaymentData<api::Session>
{
    fn to_construct_r_d(
        &self,
    ) -> RouterResult<
        &(dyn ConstructFlowSpecificData<
            api::Session,
            types::PaymentsSessionData,
            types::PaymentsSessionResponseData,
        > + Send
              + Sync),
    > {
        Ok(self)
    }
}

impl DecideFlow<api::Session, types::PaymentsSessionData, types::PaymentsSessionResponseData>
    for types::RouterData<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsSessionResponseData,
    >
{
    fn to_decide_flows(
        &self,
    ) -> RouterResult<
        &(dyn Feature<api::Session, types::PaymentsSessionData, types::PaymentsSessionResponseData>
              + Send
              + Sync),
    > {
        Ok(self)
    }
}

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsSessionResponseData,
    > for PaymentData<api::Session>
{
    async fn construct_r_d<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::PaymentsSessionRouterData> {
        let output = transformers::construct_payment_router_data::<
            api::Session,
            types::PaymentsSessionData,
            types::PaymentsSessionResponseData,
        >(state, self.clone(), connector_id, merchant_account)
        .await?;
        Ok(output.1)
    }
}

#[async_trait]
impl Feature<api::Session, types::PaymentsSessionData, types::PaymentsSessionResponseData>
    for types::PaymentsSessionRouterData
{
    async fn decide_flows<'a>(
        &self,
        state: &AppState,
        connector: api::ConnectorData,
        customer: &Option<api::CustomerResponse>,
        payment_data: PaymentData<api::Session>,
        call_connector_action: payments::CallConnectorAction,
    ) -> (RouterResult<Self>, PaymentData<api::Session>) {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
            )
            .await;

        (resp, payment_data)
    }
}

impl types::PaymentsSessionRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: api::ConnectorData,
        _maybe_customer: &Option<api::CustomerResponse>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<types::PaymentsSessionRouterData>
    where
        dyn api::Connector + Sync: services::ConnectorIntegration<
            api::Session,
            PaymentsSessionData,
            PaymentsSessionResponseData,
        >,
    {
        let connector_integration: services::BoxedConnectorIntegration<
            api::Session,
            PaymentsSessionData,
            PaymentsSessionResponseData,
        > = connector.connector.get_connector_integration();
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            self,
            call_connector_action,
        )
        .await
        .map_err(|error| error.to_payment_failed_response())?;

        Ok(resp)
    }
}
